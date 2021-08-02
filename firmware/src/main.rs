#![no_main]
#![no_std]

// set the panic handler
extern crate panic_halt;

use cortex_m_rt::{entry};
use stm32f1xx_hal::{prelude::*, stm32, spi, serial, delay::Delay};
use nb::block;

use rtic::app;
use rtic::cyccnt::{Duration};

use ili9341::{Ili9341, Orientation};
use embedded_graphics::{
  mono_font::MonoTextStyle,
  pixelcolor::Rgb565,
  prelude::*,
  text::{Baseline, Text, TextStyle},
};
use display_interface_spi::SPIInterface;

type Display = ili9341::Ili9341<display_interface_spi::SPIInterface<stm32f1xx_hal::spi::Spi<stm32f1xx_hal::pac::SPI1, stm32f1xx_hal::spi::Spi1NoRemap, (stm32f1xx_hal::gpio::gpioa::PA5<stm32f1xx_hal::gpio::Alternate<stm32f1xx_hal::gpio::PushPull>>, stm32f1xx_hal::gpio::gpioa::PA6<stm32f1xx_hal::gpio::Input<stm32f1xx_hal::gpio::Floating>>, stm32f1xx_hal::gpio::gpioa::PA7<stm32f1xx_hal::gpio::Alternate<stm32f1xx_hal::gpio::PushPull>>), u8>, stm32f1xx_hal::gpio::gpioa::PA3<stm32f1xx_hal::gpio::Output<stm32f1xx_hal::gpio::PushPull>>, stm32f1xx_hal::gpio::gpioa::PA2<stm32f1xx_hal::gpio::Output<stm32f1xx_hal::gpio::PushPull>>>,stm32f1xx_hal::gpio::gpioa::PA4<stm32f1xx_hal::gpio::Output<stm32f1xx_hal::gpio::PushPull>>>;
type Serial = stm32f1xx_hal::serial::Serial<stm32f1xx_hal::pac::USART1, (stm32f1xx_hal::gpio::gpioa::PA9<stm32f1xx_hal::gpio::Alternate<stm32f1xx_hal::gpio::PushPull>>, stm32f1xx_hal::gpio::gpioa::PA10<stm32f1xx_hal::gpio::Input<stm32f1xx_hal::gpio::Floating>>)>;


#[app(device = stm32f1xx_hal::pac, peripherals = true)]
const APP: () = {
  struct Resources {
    display: Display,
    serial: Serial,
    gps: Option<nmea0183::GGA>
  }

  #[init(schedule = [])]
  fn init(mut cx: init::Context) -> init::LateResources {
    cx.core.DCB.enable_trace();
    // required on Cortex-M7 devices that software lock the DWT (e.g. STM32F7)
    cortex_m::peripheral::DWT::unlock();
    cx.core.DWT.enable_cycle_counter();

    // Take ownership over the raw flash and rcc devices and convert them into the corresponding
    // HAL structs
    let mut flash = cx.device.FLASH.constrain();
    let mut rcc = cx.device.RCC.constrain();

    let clocks = rcc
      .cfgr
      .use_hse(8.mhz())
      .sysclk(32.mhz())
      .freeze(&mut flash.acr);

    let mut afio = cx.device.AFIO.constrain(&mut rcc.apb2);

    // configure the gpio ports
    let mut gpioa = cx.device.GPIOA.split(&mut rcc.apb2);

    // configure the display driver
    let cs    = gpioa.pa2.into_push_pull_output(&mut gpioa.crl);
    let dc    = gpioa.pa3.into_push_pull_output(&mut gpioa.crl);
    let reset = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);
    let sck   = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
    let miso  = gpioa.pa6;
    let mosi  = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);
    let spi   = spi::Spi::spi1(
        cx.device.SPI1,
        (sck, miso, mosi),
        &mut afio.mapr,
        ili9341::SPI_MODE,
        16.mhz(),
        clocks,
        &mut rcc.apb2,
    );
    let mut delay = Delay::new(cx.core.SYST, clocks);
    let display = Ili9341::new(
      SPIInterface::new(spi, dc, cs),
      reset,
      &mut delay,
      Orientation::Landscape,
      ili9341::DisplaySize320x480,
      ).unwrap();

    // Configure the serial port for GPS data
    let tx = gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh);
    let rx = gpioa.pa10;

    let mut serial = serial::Serial::usart1(
      cx.device.USART1,
      (tx, rx),
      &mut afio.mapr,
      serial::Config::default().baudrate(9600.bps()),
      clocks,
      &mut rcc.apb2,
    );
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
        Ok(nmea0183::ParseResult::GGA(Some(gga))) => {
          *cx.resources.gps = Option::from(gga);
        },
        Ok(_) => {},
        Err(_) => {},
      }
    }
  }

  #[idle(resources=[gps,display])]
  fn idle(mut cx: idle::Context) -> ! {
    let character_style = MonoTextStyle::new(&profont::PROFONT_12_POINT, Rgb565::WHITE);
    let text_style = TextStyle::with_baseline(Baseline::Top);
    let cursor: Point = Point::zero();

    loop {
      cortex_m::asm::wfi();


      // Fetch the updated GPS value, if there is one
      let mut gga: Option<nmea0183::GGA> = Option::None;
      cx.resources.gps.lock( |gps| {
        gga = gps.take();        
      });

      // And show it
      if let Some(gga) = gga {
        let content = "GGA"; 
        Text::with_text_style(content, cursor, character_style, text_style)
        .draw(cx.resources.display).unwrap();
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
