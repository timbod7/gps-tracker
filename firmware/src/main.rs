#![no_main]
#![no_std]

mod u8writer;

use embedded_graphics::mono_font::MonoTextStyleBuilder;
use rtt_target::{rtt_init_print, rprintln};

// set the panic handler
extern crate panic_rtt_target;

use stm32f4xx_hal::{prelude::*, spi, serial, delay::Delay};
use nb::block;

use rtic::app;

use ili9341::{Ili9341, Orientation};
use embedded_graphics::{
  mono_font::MonoTextStyle,
  pixelcolor::Rgb565,
  prelude::*,
  text::{Baseline, Text, TextStyle},
  primitives::{Rectangle, PrimitiveStyle},
};
use display_interface_spi::SPIInterface;
use u8writer::U8Writer;
use core::fmt::{Write};


type Display = ili9341::Ili9341<display_interface_spi::SPIInterface<stm32f4xx_hal::spi::Spi<stm32f4xx_hal::stm32::SPI1, (stm32f4xx_hal::gpio::gpioa::PA5<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF5>>, stm32f4xx_hal::gpio::gpioa::PA6<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF5>>, stm32f4xx_hal::gpio::gpioa::PA7<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF5>>)>, stm32f4xx_hal::gpio::gpioa::PA3<stm32f4xx_hal::gpio::Output<stm32f4xx_hal::gpio::PushPull>>, stm32f4xx_hal::gpio::gpioa::PA2<stm32f4xx_hal::gpio::Output<stm32f4xx_hal::gpio::PushPull>>>, stm32f4xx_hal::gpio::gpioa::PA4<stm32f4xx_hal::gpio::Output<stm32f4xx_hal::gpio::PushPull>>>;
type Serial = stm32f4xx_hal::serial::Serial<stm32f4xx_hal::stm32::USART1, (stm32f4xx_hal::gpio::gpioa::PA9<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF7>>, stm32f4xx_hal::gpio::gpioa::PA10<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF7>>)>;

#[app(device = stm32f4xx_hal::pac, peripherals = true)]
const APP: () = {
  struct Resources {
    display: Display,
    serial: Serial,
    gps: Option<Option<nmea0183::GGA>>
  }

  #[init(schedule = [])]
  fn init(cx: init::Context) -> init::LateResources {
    rtt_init_print!();
    rprintln!("Starting init");
    rprintln!("Starting init2");

    rprintln!("Starting init2.5");


    // Take ownership over the raw flash and rcc devices and convert them into the corresponding
    // HAL structs
    let rcc = cx.device.RCC.constrain();

    let clocks = rcc
      .cfgr
      .use_hse(25.mhz())
      .sysclk(32.mhz())
      .freeze();

    // Enable debugging in sleep modes so that stlink stays alive during wfi etc._
    // Remove this if/when power consumption is an issue.
    cx.device.DBGMCU.cr.write( |w| w
      .dbg_sleep().set_bit()
      .dbg_stop().set_bit()
      .dbg_standby().set_bit()
    );

    rprintln!("Starting init3");


    // configure the gpio ports
    let gpioa = cx.device.GPIOA.split();

    // configure the display driver
    let cs    = gpioa.pa2.into_push_pull_output();
    let dc    = gpioa.pa3.into_push_pull_output();
    let reset = gpioa.pa4.into_push_pull_output();
    let sck   = gpioa.pa5.into_alternate_af5();
    let miso  = gpioa.pa6.into_alternate_af5();
    let mosi  = gpioa.pa7.into_alternate_af5();
    let spi   = spi::Spi::spi1(
        cx.device.SPI1,
        (sck, miso, mosi),
        ili9341::SPI_MODE,
        16_000_000.hz(),
        clocks,
    );
    let mut delay = Delay::new(cx.core.SYST, clocks);
    let display = Ili9341::new(
      SPIInterface::new(spi, dc, cs),
      reset,
      &mut delay,
      Orientation::Landscape,
      ili9341::DisplaySize320x480,
      ).unwrap();

    rprintln!("Starting init4");

    // Configure the serial port for GPS data
    let tx = gpioa.pa9.into_alternate_af7();
    let rx = gpioa.pa10.into_alternate_af7();

    let mut serial = serial::Serial::usart1(
      cx.device.USART1,
      (tx, rx),
      serial::config::Config::default().baudrate(9600.bps()),
      clocks
    ).unwrap();
    serial.listen(serial::Event::Rxne);

    init::LateResources {
      display,
      serial,
      gps: Option::None,
    }
  }

  #[task(binds = USART1, resources=[serial, gps])]
  fn usart1(cx: usart1::Context) {
    static mut PARSER: Option<nmea0183::Parser> = None;

    let parser = PARSER.get_or_insert_with(|| {
      nmea0183::Parser::new()
    });

    let received: u8 = block!(cx.resources.serial.read()).unwrap();

    if let Some(result) = parser.parse_from_byte(received) {
      match result {
        Ok(nmea0183::ParseResult::GGA(msg)) => {
          rprintln!("usart1: GGA {}", msg.is_some());
          *cx.resources.gps = Option::from(msg)
        },
        Ok(nmea0183::ParseResult::GLL(msg)) => {
          rprintln!("usart1: GLL {}", msg.is_some());
        },
        Ok(nmea0183::ParseResult::RMC(msg)) => {
          rprintln!("usart1: RMC {}", msg.is_some());
  
        },
        Ok(nmea0183::ParseResult::VTG(msg)) => {
          rprintln!("usart1: VTG {}", msg.is_some());
        }
        Err(_) => {},
      }
    }
  }

  #[idle(resources=[gps,display])]
  fn idle(mut cx: idle::Context) -> ! {
    let character_style = MonoTextStyle::new(&profont::PROFONT_18_POINT, Rgb565::WHITE);
    let text_style = TextStyle::with_baseline(Baseline::Top);
    let mut nmissing:u32 = 0;

    write_lcd_line(cx.resources.display, 0,"GPS");

    loop {
      //rprintln!("loop");

      // Fetch the updated GPS value, if there is one
      let mut oogga: Option<Option<nmea0183::GGA>> = Option::None;
      cx.resources.gps.lock( |gps| {
        oogga = gps.take();        
      });

      let mut buf = [0u8; 20];
      let mut buf = U8Writer::new(&mut buf[..]);

      // And show it
      if let Some(ogga) = oogga {
        if let Some(gga) = ogga {
          buf.clear();
          write!(&mut buf, "Sats: {}", gga.sat_in_use).unwrap();
          write_lcd_line(cx.resources.display, 1, buf.as_str());

          buf.clear();
          write!(&mut buf, "Lat: {}", gga.latitude.as_f64()).unwrap();
          write_lcd_line(cx.resources.display, 2, buf.as_str());

          buf.clear();
          write!(&mut buf, "Long: {}", gga.longitude.as_f64()).unwrap();
          write_lcd_line(cx.resources.display, 3, buf.as_str());

        } else {
          nmissing = nmissing + 1;
          buf.clear();
          write!(&mut buf, "Sats: 0 (nm: {})", nmissing).unwrap();
          write_lcd_line(cx.resources.display, 1, buf.as_str())
        }
      }
    }
  }


  // RTIC requires that unused interrupts are declared in an extern block when
  // using software tasks; these free interrupts will be used to dispatch the
  // software tasks.
  extern "C" {
      fn USART3();
  }
};


fn write_lcd_line(display: &mut Display, line: i32, content: &str) {
  static LINE_SIZE: i32 = 22;
  let char_style = MonoTextStyleBuilder::new()
    .font(&profont::PROFONT_18_POINT)
    .text_color(Rgb565::WHITE)
    .background_color(Rgb565::RED)
    .build();
  let text_style = TextStyle::with_baseline(Baseline::Top);
  let origin = Point::new(0,LINE_SIZE * line);
  Text::with_text_style(content, origin, char_style, text_style)
    .draw(display).unwrap();
}
