use rtt_target::{rprintln};


pub struct Gps {
  parser: nmea0183::Parser,
  gga: Option<Option<nmea0183::GGA>>,
  vtg: Option<nmea0183::VTG>,

}

impl Gps {
  pub fn new() -> Gps {
    Gps{
      parser: nmea0183::Parser::new(),
      gga: Option::None,
      vtg: Option::None
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
            self.vtg.replace(vtg);
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
}