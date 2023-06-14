use embedded_hal::serial;
use nb::block;
use rtt_target::rprintln;

// How often we receive position data
const GPS_MESSAGE_MS: u16 = 500;

// How many speed samples we record to get an
// average over a second
const SPEED_SAMPLES: usize = 1000 / (GPS_MESSAGE_MS as usize);

// How many speed samples we record to get an
// average over 10 seconds
const SPEED_AVG_SAMPLES: usize = 10 * 1000 / (GPS_MESSAGE_MS as usize);

#[derive(Clone)]
pub struct GpsData {
    pub sat_in_use: u8,
    pub course: Option<f32>,
    pub speed: f32,
    pub max_speed: f32,
    pub avg_speed: f32,
    pub max_avg_speed: f32,
    pub distance_m: u32,
    pub hdop: Option<f32>,
    pub latitude: Option<f32>,
    pub longitude: Option<f32>,
    pub time: Option<GpsTime>,
}

#[derive(Clone)]
pub struct GpsTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub min: u8,
    pub sec: u8,
}

pub struct Gps {
    parser: ublox::Parser<GpsBuffer>,

    output: GpsData,

    updated: Option<()>, // atomic bool???

    speed_samples: AverageBuffer<SPEED_SAMPLES>,
    avg_speed_samples: AverageBuffer<SPEED_AVG_SAMPLES>,
}

impl Gps {
    pub fn new() -> Gps {
        let buf = GpsBuffer::new();
        let parser = ublox::Parser::new(buf);

        Gps {
            parser,
            output: GpsData {
                sat_in_use: 0,
                course: None,
                hdop: None,
                latitude: None,
                longitude: None,
                speed: 0f32,
                distance_m: 0,
                time: None,
                max_speed: 0f32,
                avg_speed: 0f32,
                max_avg_speed: 0f32,
            },

            updated: Option::Some(()),
            speed_samples: AverageBuffer::new(),
            avg_speed_samples: AverageBuffer::new(),
        }
    }

    pub fn init<S: serial::Write<u8> + serial::Read<u8>>(&mut self, serial: &mut S) {
        loop {
            // Wait a bit to give the GPS some time for a cold start
            cortex_m::asm::delay(16_000_000);

            match self.init0(serial) {
                Ok(_) => break,
                Err(_) => {}
            }

            // Reset the parser
            let buf = GpsBuffer::new();
            self.parser = ublox::Parser::new(buf);
        }
    }

    pub fn init0<S: serial::Write<u8> + serial::Read<u8>>(
        &mut self,
        serial: &mut S,
    ) -> Result<(), ()> {
        use ublox::*;
        rprintln!("gps: init");

        // Configure to talk UBX
        rprintln!("gps: use UBX 1/2");
        let msg = CfgPrtUartBuilder {
            portid: UartPortId::Uart1,
            reserved0: 0,
            tx_ready: 0,
            mode: UartMode::new(DataBits::Eight, Parity::None, StopBits::One),
            baud_rate: 9600,
            in_proto_mask: InProtoMask::all(),
            out_proto_mask: OutProtoMask::UBLOX,
            flags: 0,
            reserved5: 0,
        }
        .into_packet_bytes();
        self.serial_write(serial, &msg);

        // Wait a bit
        cortex_m::asm::delay(16_000_000);
        // Throw away rx contents
        let _ = serial.read();

        // Send the message again
        rprintln!("gps: use UBX 2/2");
        self.serial_write(serial, &msg);

        rprintln!("gps: awaiting ack for UBX");
        self.serial_wait_for_ack::<S, CfgPrtUart>(serial)?;

        // Set the measurement/nav rate to 2 Hz
        rprintln!("gps: set rate to 2Hz");
        let msg = CfgRateBuilder {
            measure_rate_ms: GPS_MESSAGE_MS,
            nav_rate: 1,
            time_ref: AlignmentToReferenceTime::Utc,
        }
        .into_packet_bytes();
        self.serial_write(serial, &msg);
        self.serial_wait_for_ack::<S, CfgRate>(serial)?;

        // Enable the packets required
        rprintln!("gps: enable NavPosVelTime");
        let msg = CfgMsgAllPortsBuilder::set_rate_for::<NavPosVelTime>([0, 1, 0, 0, 0, 0])
            .into_packet_bytes();
        self.serial_write(serial, &msg);
        self.serial_wait_for_ack::<S, CfgMsgAllPorts>(serial)?;
        rprintln!("gps: enable NavOdo");
        let msg =
            CfgMsgAllPortsBuilder::set_rate_for::<NavOdo>([0, 1, 0, 0, 0, 0]).into_packet_bytes();
        self.serial_write(serial, &msg);
        self.serial_wait_for_ack::<S, CfgMsgAllPorts>(serial)?;

        // Send a packet request for the MonVer packet
        rprintln!("gps: request MonVer");
        let msg = UbxPacketRequest::request_for::<MonVer>().into_packet_bytes();
        self.serial_write(serial, &msg);

        Ok(())
    }

    fn serial_write<S: serial::Write<u8>>(&mut self, serial: &mut S, msg: &[u8]) {
        for b in msg {
            let _ = block!(serial.write(*b));
        }
        let _ = block!(serial.flush());
    }

    fn serial_wait_for_ack<S: serial::Read<u8>, T: ublox::UbxPacketMeta>(
        &mut self,
        serial: &mut S,
    ) -> Result<(), ()> {
        #[derive(PartialEq)]
        enum State {
            Waiting,
            Found,
            Failed,
        }

        let mut state = State::Waiting;
        while state == State::Waiting {
            let ec = block!(serial.read());
            match ec {
                Result::Err(_e) => {
                    rprintln!("rx fail waiting for ack");
                    state = State::Failed;
                }
                Result::Ok(c) => {
                    let buf = [c];
                    let mut it = self.parser.consume(&buf);
                    loop {
                        match it.next() {
                            Some(Ok(ublox::PacketRef::AckAck(ack))) => {
                                if ack.class() == T::CLASS && ack.msg_id() == T::ID {
                                    state = State::Found;
                                } else {
                                    rprintln!("ignoring other ack");
                                    state = State::Failed;
                                }
                            }
                            Some(Ok(_)) => {
                                rprintln!("ignoring other message");
                                state = State::Failed;
                            }
                            Some(Err(_)) => {
                                rprintln!("ignoring parse error");
                                state = State::Failed;
                            }
                            None => {
                                break;
                            }
                        }
                    }
                }
            }
        }
        if state == State::Found {
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn parse_clear(&mut self) {
        rprintln!("gps: parse clear");
        let buf = GpsBuffer::new();
        self.parser = ublox::Parser::new(buf);
    }

    pub fn parse_u8(&mut self, received: u8) {
        let nb = [received; 1];

        let mut it = self.parser.consume(&nb);
        loop {
            match it.next() {
                Some(Ok(ublox::PacketRef::NavPosVelTime(sol))) => {
                    rprintln!("gps: NavPosVelTime {}", sol.flags().bits());
                    self.output.sat_in_use = sol.num_satellites();
                    if sol.flags().contains(ublox::NavPosVelTimeFlags::GPS_FIX_OK) {
                        self.output.hdop = None;
                        self.output.latitude = Some(degrees_from_raw(sol.lat_degrees_raw()));
                        self.output.longitude = Some(degrees_from_raw(sol.lon_degrees_raw()));
                        self.output.course = Some(heading_from_raw(sol.heading_degrees_raw()));
                        self.output.time = Some(GpsTime {
                            year: sol.year(),
                            month: sol.month(),
                            day: sol.day(),
                            hour: sol.hour(),
                            min: sol.min(),
                            sec: sol.sec(),
                        });
                    } else {
                        self.output.hdop = None;
                        self.output.latitude = None;
                        self.output.longitude = None;
                        self.output.course = None;
                        self.output.time = None;
                        self.speed_samples = AverageBuffer::new();
                        self.avg_speed_samples = AverageBuffer::new();
                    }
                    let raw_speed = knots_from_raw(sol.ground_speed_raw());

                    self.speed_samples.add(raw_speed);
                    self.output.speed = self.speed_samples.avg_value();
                    update_max(&mut self.output.max_speed, self.output.speed);

                    self.avg_speed_samples.add(raw_speed);
                    self.output.avg_speed = self.avg_speed_samples.avg_value();
                    update_max(&mut self.output.max_avg_speed, self.output.avg_speed);
                }
                Some(Ok(ublox::PacketRef::MonVer(monver))) => {
                    rprintln!(
                        "gps: MonVer {}/{}",
                        monver.hardware_version(),
                        monver.software_version()
                    );
                }
                Some(Ok(ublox::PacketRef::NavOdo(odo))) => {
                    self.output.distance_m = odo.distance();
                    self.updated = Some(());
                }
                Some(Ok(_)) => {
                    rprintln!("gps: ignored unused packet");
                    // Recevied a valid packet not of interest, ignore it
                }
                Some(Err(e)) => {
                    rprintln!("gps: ignored malformed packet {:#}", e);
                    // Received a malformed packet, ignore it
                }
                None => {
                    // We've eaten all the packets we have
                    break;
                }
            }
        }
    }

    pub fn take(&mut self) -> Option<GpsData> {
        let updated = self.updated.take();
        if let Some(()) = updated {
            Some(self.output.clone())
        } else {
            None
        }
    }
}

fn update_max(max: &mut f32, v: f32) {
    if v > *max {
        *max = v;
    }
}

struct GpsBuffer {
    buffer: [u8; 192],
    len: usize,
}

impl GpsBuffer {
    fn new() -> Self {
        GpsBuffer {
            buffer: [0; 192],
            len: 0,
        }
    }
}

impl core::ops::Index<usize> for GpsBuffer {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.buffer[index]
    }
}

impl core::ops::Index<core::ops::Range<usize>> for GpsBuffer {
    type Output = [u8];

    fn index(&self, index: core::ops::Range<usize>) -> &Self::Output {
        if index.end > self.len {
            panic!("Index {} is outside of our length {}", index.end, self.len);
        }
        self.buffer.index(index)
    }
}

impl ublox::UnderlyingBuffer for GpsBuffer {
    fn clear(&mut self) {
        self.len = 0;
    }

    fn len(&self) -> usize {
        self.len
    }

    fn max_capacity(&self) -> usize {
        self.buffer.len()
    }

    fn extend_from_slice(&mut self, other: &[u8]) -> usize {
        let to_copy = core::cmp::min(other.len(), self.buffer.len() - self.len);
        let uncopyable = other.len() - to_copy;
        self.buffer[self.len..self.len + to_copy].copy_from_slice(&other[..to_copy]);
        self.len += to_copy;
        uncopyable
    }

    fn drain(&mut self, count: usize) {
        if count >= self.len {
            self.len = 0;
            return;
        }

        let new_size = self.len - count;
        {
            let bufptr = self.buffer.as_mut_ptr();
            unsafe {
                core::ptr::copy(bufptr.add(count), bufptr, new_size);
            }
        }
        self.len = new_size;
    }

    fn find(&self, value: u8) -> Option<usize> {
        for i in 0..self.len {
            if self.buffer[i] == value {
                return Some(i);
            }
        }
        None
    }
}

fn degrees_from_raw(raw: i32) -> f32 {
    raw as f32 * 1e-7
}

fn heading_from_raw(raw: i32) -> f32 {
    raw as f32 * 1e-5
}

fn knots_from_raw(raw: u32) -> f32 {
    raw as f32 * 1e-3 * 1.943844
}

pub struct AverageBuffer<const N: usize> {
    samples: [f32; N],
    si: usize,
}

impl<const N: usize> AverageBuffer<N> {
    pub fn new() -> Self {
        AverageBuffer {
            samples: [0f32; N],
            si: 0,
        }
    }

    pub fn add(&mut self, v: f32) {
        self.samples[self.si] = v;
        self.si = (self.si + 1) % N;
    }

    pub fn avg_value(&self) -> f32 {
        self.samples.iter().fold(0f32, |v1, v2| v1 + v2) / (N as f32)
    }
}
