use rtt_target::{rprintln};

const SPEED_AVG_SAMPLES: usize = 5;

use nmea0183::coords::Speed;

pub struct Gps {
  parser: nmea0183::Parser,
  gga: Option<Option<nmea0183::GGA>>,
  vtg: Option<nmea0183::VTG>,

  avg_speed_samples: [f32; SPEED_AVG_SAMPLES],
  avg_i: usize,

}

impl Gps {
  pub fn new() -> Gps {
    
    Gps{
      parser: nmea0183::Parser::new(),
      gga: Option::None,
      vtg: Option::None,
      avg_speed_samples: [0f32; SPEED_AVG_SAMPLES],
      avg_i: 0,
    }
  }

  pub fn parse_clear(&mut self) {
    self.parser = nmea0183::Parser::new()
  }

  pub fn parse_u8(&mut self, received: u8) {
    if let Some(result) = self.parser.parse_from_byte(received) {
      match result {
        Ok(nmea0183::ParseResult::GGA(msg)) => {
          rprintln!("usart1: GGA {}", msg.is_some());
          self.gga.replace(Option::from(msg));
        },
        Ok(nmea0183::ParseResult::GLL(msg)) => {
          rprintln!("usart1: GLL {}", msg.is_some());
        },
        Ok(nmea0183::ParseResult::RMC(msg)) => {
          rprintln!("usart1: RMC {}", msg.is_some());
  
        },
        Ok(nmea0183::ParseResult::VTG(msg)) => {
          rprintln!("usart1: VTG {}", msg.is_some());
          if let Option::Some(vtg) = msg {
            self.avg_speed_samples[self.avg_i] = vtg.speed.as_knots();
            self.avg_i = (self.avg_i + 1) % SPEED_AVG_SAMPLES;
            self.vtg.replace(vtg);
          } else {
            self.avg_speed_samples = [0f32; SPEED_AVG_SAMPLES];
          }
        }
        Err(_) => {},
      }
    }
  }

  pub fn take_gga(&mut self) -> Option<Option<nmea0183::GGA>> {
    self.gga.take()
  }

  pub fn take_vtg(&mut self) -> Option<nmea0183::VTG> {
    self.vtg.take()
  }

  pub fn avg_speed(&self) -> Speed {
    let mut total = 0f32;
    for i in 0..SPEED_AVG_SAMPLES {
      total += self.avg_speed_samples[i];
    }
    return  Speed::from_knots(total / SPEED_AVG_SAMPLES as f32);
  }
}