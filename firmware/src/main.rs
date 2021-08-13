#![no_main]
#![no_std]
#[allow(dead_code)]

mod u8writer;
mod layout;
mod gps;

use rtt_target::{rtt_init_print, rprintln};

// set the panic handler
extern crate panic_rtt_target;

use stm32f4xx_hal::{prelude::*, spi, serial, delay::Delay};
use nb::block;

use rtic::app;

use ili9341::{Ili9341, Orientation};

use display_interface_spi::SPIInterface;
use u8writer::U8Writer;
use gps::Gps;
use nmea0183::coords::Speed;

type Display = ili9341::Ili9341<display_interface_spi::SPIInterface<stm32f4xx_hal::spi::Spi<stm32f4xx_hal::stm32::SPI1, (stm32f4xx_hal::gpio::gpioa::PA5<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF5>>, stm32f4xx_hal::gpio::gpioa::PA6<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF5>>, stm32f4xx_hal::gpio::gpioa::PA7<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF5>>)>, stm32f4xx_hal::gpio::gpioa::PA3<stm32f4xx_hal::gpio::Output<stm32f4xx_hal::gpio::PushPull>>, stm32f4xx_hal::gpio::gpioa::PA2<stm32f4xx_hal::gpio::Output<stm32f4xx_hal::gpio::PushPull>>>, stm32f4xx_hal::gpio::gpioa::PA4<stm32f4xx_hal::gpio::Output<stm32f4xx_hal::gpio::PushPull>>>;
type Serial = stm32f4xx_hal::serial::Serial<stm32f4xx_hal::stm32::USART1, (stm32f4xx_hal::gpio::gpioa::PA9<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF7>>, stm32f4xx_hal::gpio::gpioa::PA10<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF7>>)>;

#[app(device = stm32f4xx_hal::pac, peripherals = true)]
const APP: () = {
  struct Resources {
    display: Display,
    serial: Serial,
    gps: Gps,
  }

  #[init(schedule = [])]
  fn init(cx: init::Context) -> init::LateResources {
    rtt_init_print!();
    rprintln!("Starting init");

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
      gps: Gps::new(),
    }
  }

  #[task(binds = USART1, resources=[serial, gps])]
  fn usart1(cx: usart1::Context) {
    let ereceived: Result<u8,serial::Error> = block!(cx.resources.serial.read());
    match ereceived {
      Result::Err(_e) => {
        rprintln!("usart1: error");
        cx.resources.gps.parse_clear();
      },
      Result::Ok(received) => {
        cx.resources.gps.parse_u8(received);
      },
    }
  }

  #[idle(resources=[gps,display])]
  fn idle(mut cx: idle::Context) -> ! {
    let mut screen = layout::Screen1::new();

    screen.render_initial(cx.resources.display).unwrap();

    loop {
      // Fetch the updated GGA and VTG values, if present
      let mut oogga: Option<Option<nmea0183::GGA>> = Option::None;
      let mut ovtg: Option<nmea0183::VTG> = Option::None;
      let mut avg_speed = Speed::from_knots(0f32);
      
      cx.resources.gps.lock( |gps| {
        oogga = gps.take_gga();
        ovtg = gps.take_vtg();
        if ovtg.is_some() {
          avg_speed = gps.avg_speed();
        }
      });

      let mut updated = false;
      if let Some(ogga) = oogga {
        screen.update_gga(ogga);
        updated = true;
      }

      if let Some(vtg) = ovtg {
        screen.update_vtg(vtg, avg_speed);
        updated = true;
      }

      if updated {
        screen.render_update(cx.resources.display).unwrap();
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

