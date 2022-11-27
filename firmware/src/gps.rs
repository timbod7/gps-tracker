
use rtt_target::{rprintln};
use embedded_hal::serial;
use nb::block;


const SPEED_AVG_SAMPLES: usize = 5;

pub struct GpsData {
  pub sat_in_use: u8,
  pub course: Option<f32>,
  pub speed: f32,
  pub avg_speed: f32,
  pub max_speed: f32,
  pub hdop: Option<f32>,
  pub latitude: Option<f32>,
  pub longitude: Option<f32>,
}

pub struct Gps {

  parser: ublox::Parser<GpsBuffer>,
  // dbuf: [u8; 32],
  // dbufi: usize,

  updated: Option<()>,  // atomic bool???

  sat_in_use: u8,
  speed: f32,
  course: Option<f32>,
  hdop: Option<f32>,
  latitude: Option<f32>,
  longitude: Option<f32>,
  avg_speed_samples: [f32; SPEED_AVG_SAMPLES],
  avg_i: usize,
  max_speed: f32,
}

impl Gps {
  pub fn new() -> Gps {
    let buf = GpsBuffer::new();
    let parser = ublox::Parser::new(buf);

    Gps{
      parser,
      // dbuf: [0; 32],
      // dbufi: 0,
      updated: Option::Some(()),
      sat_in_use : 0,
      speed: 0f32,
      course: None,
      hdop: None,
      latitude: None,
      longitude: None,
      avg_speed_samples: [0f32; SPEED_AVG_SAMPLES],
      avg_i: 0,
      max_speed: 0f32,
    }
  }

  pub fn init<S: serial::Write<u8> + serial::Read<u8> >(&mut self, serial: &mut S) {
      use ublox::*;

      // Configure to talk UBX
      rprintln!("gps: use UBX");
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
    }.into_packet_bytes();
    self.serial_write(serial, &msg);
    self.serial_wait_for_ack::<S,CfgPrtUart>(serial);

    // Set the measurement/nav rate to 2 Hz
    rprintln!("gps: set rate to 2Hz");
    let msg = CfgRateBuilder {
                measure_rate_ms: 500,
                nav_rate: 1,
                time_ref: AlignmentToReferenceTime::Utc,
            }
            .into_packet_bytes();
    self.serial_write(serial, &msg);
    self.serial_wait_for_ack::<S,CfgRate>(serial);

    // Enable the NavPosVelTime packet
    rprintln!("gps: enable NavPosVelTime");
    let msg = CfgMsgAllPortsBuilder::set_rate_for::<NavPosVelTime>([0, 1, 0, 0, 0, 0])
                .into_packet_bytes();
    self.serial_write(serial, &msg);
    self.serial_wait_for_ack::<S,CfgMsgAllPorts>(serial);

    // Send a packet request for the MonVer packet
    rprintln!("gps: request MonVer");
    let msg = UbxPacketRequest::request_for::<MonVer>().into_packet_bytes();
    self.serial_write(serial, &msg);

    // self.serial_test_run(serial);
  }

  fn serial_write<S: serial::Write<u8>>(&mut self, serial: &mut S, msg: &[u8]) {
    for b in msg {
      block!(serial.write(*b));
    }
    block!(serial.flush());
  }

  fn serial_wait_for_ack<S: serial::Read<u8>, T: ublox::UbxPacketMeta>(&mut self, serial: &mut S) {
    let mut found_ack = false;
    while !found_ack {
      let ec = block!(serial.read());
      match ec {
        Result::Err(_e) => {
          rprintln!("rx fail waiting for ack");
        },
        Result::Ok(c) => {
          let buf = [c];
          let mut it = self.parser.consume(&buf);
          loop {
          match it.next() {
              Some(Ok(ublox::PacketRef::AckAck(ack))) => {
                if ack.class() == T::CLASS && ack.msg_id() == T::ID {
                  found_ack = true;
                } else {
                  rprintln!("ignoring other ack");
                }
              }
              Some(Ok(_)) => {
                rprintln!("ignoring other message");
              }
              Some(Err(_)) => {
                rprintln!("ignoring parse error");
              }
              None => {break;}
            }
          }
        },
      }
    }
    rprintln!("gps: received ack");
  }

  fn serial_test_run<S: serial::Read<u8>>(&mut self, serial: &mut S) {
      loop {
      let ec = block!(serial.read());
      match ec {
        Result::Err(_e) => {
          rprintln!("rx fail waiting for packet");
        },
        Result::Ok(c) => {
          self.parse_u8(c);
        },
      }
    }
  }

  pub fn parse_clear(&mut self) {
    rprintln!("gps: parse clear");
    let buf = GpsBuffer::new();
    self.parser = ublox::Parser::new(buf);  
  }

  pub fn parse_u8(&mut self, received: u8) {
    
    // self.dbuf[self.dbufi] = received;
    // self.dbufi += 1;
    // if (self.dbufi == self.dbuf.len()) {
    //   rprintln!("rx: {:?}", self.dbuf);
    //   self.dbufi = 0;
    // }

    let nb = [received; 1];

    let mut it = self.parser.consume(&nb);
    loop {
      match it.next() {
          Some(Ok(ublox::PacketRef::NavPosVelTime(sol))) => {
            rprintln!("gps: NavPosVelTime {}", sol.flags().bits());
            self.sat_in_use = sol.num_satellites();
            if sol.flags().contains(ublox::NavPosVelTimeFlags::GPS_FIX_OK) {
              self.hdop = None;
              self.latitude = Some(degrees_from_raw(sol.lat_degrees_raw()));
              self.longitude = Some(degrees_from_raw(sol.lon_degrees_raw()));
              self.course = Some(heading_from_raw(sol.heading_degrees_raw()));
            } else {
              self.hdop = None;
              self.latitude = None;
              self.longitude = None;
              self.course = None;
              self.avg_speed_samples = [0f32; SPEED_AVG_SAMPLES];
            }
            self.speed = knots_from_raw(sol.ground_speed_raw());
            self.avg_speed_samples[self.avg_i] = self.speed;
            self.avg_i = (self.avg_i + 1) % SPEED_AVG_SAMPLES;
            if self.speed > self.max_speed {
              self.max_speed = self.speed;
            }
            self.updated = Some(());
          }
          Some(Ok(ublox::PacketRef::MonVer(monver))) => {
              rprintln!("gps: MonVer {}/{}", monver.hardware_version(), monver.software_version());
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
      let mut total = 0f32;
      for i in 0..SPEED_AVG_SAMPLES {
        total += self.avg_speed_samples[i];
      }

      Some(GpsData {
        sat_in_use: self.sat_in_use, 
        course: self.course,
        hdop: self.hdop,
        latitude: self.latitude,
        longitude: self.longitude,
        speed: self.speed, 
        avg_speed: total / SPEED_AVG_SAMPLES as f32, 
        max_speed: self.max_speed,
      })
    } else {
      None
    }
  }
}

struct GpsBuffer {
  buffer: [u8; 192],
  len: usize,
}

impl GpsBuffer {
  fn new() -> Self {
    GpsBuffer{ buffer: [0;192], len: 0}
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


fn degrees_from_raw(raw: i32) -> f32{
  raw as f32 * 1e-7
}

fn heading_from_raw(raw: i32) -> f32{
  raw as f32 * 1e-5
}

fn knots_from_raw(raw: u32) -> f32 {
  raw as f32 * 1e-3 * 1.943844
}