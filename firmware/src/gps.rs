use rtt_target::{rprintln};

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
  parser: nmea0183::Parser,

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
    
    Gps{
      parser: nmea0183::Parser::new(),
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

  pub fn parse_clear(&mut self) {
    self.parser = nmea0183::Parser::new()
  }

  pub fn parse_u8(&mut self, received: u8) {
    if let Some(result) = self.parser.parse_from_byte(received) {
      match result {
        Ok(nmea0183::ParseResult::GGA(Some(gga))) => {
          rprintln!("usart1: GGA sat_in_use = {}", gga.sat_in_use);
          self.sat_in_use = gga.sat_in_use;
          self.hdop = Some(gga.hdop);
          self.latitude = Some(gga.latitude.as_f64() as f32);
          self.longitude = Some(gga.longitude.as_f64() as f32);
        },
        Ok(nmea0183::ParseResult::GGA(None)) => {
          rprintln!("usart1: GGA sat_in_use = 0");
          self.sat_in_use = 0;
          self.hdop = None;
          self.latitude = None;
          self.longitude = None;
        },
        Ok(nmea0183::ParseResult::GLL(msg)) => {
        },
        Ok(nmea0183::ParseResult::RMC(msg)) => {
        },
        Ok(nmea0183::ParseResult::VTG(msg)) => {
          rprintln!("usart1: VTG {}", msg.is_some());
          if let Option::Some(vtg) = msg {
            self.course = vtg.course.map(|c| c.degrees);
            self.speed = vtg.speed.as_knots();
            self.avg_speed_samples[self.avg_i] = self.speed;
            self.avg_i = (self.avg_i + 1) % SPEED_AVG_SAMPLES;
            if vtg.speed.as_knots() > self.max_speed {
              self.max_speed = vtg.speed.as_knots();
            }
          } else {
            self.avg_speed_samples = [0f32; SPEED_AVG_SAMPLES];
          }
          self.updated = Some(());
        }
        Err(_) => {},
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