#![no_main]
#![no_std]
#[allow(dead_code)]

mod u8writer;
mod layout;
mod gps;
mod debouncer;
mod memory_display;

use rtt_target::{rtt_init_print, rprintln};

// set the panic handler
extern crate panic_rtt_target;

use embedded_hal::spi::{Mode, Phase, Polarity};
use stm32f4xx_hal::{
  gpio::{NoPin},
  prelude::*,
  serial
};

use nb::block;

use rtic::app;

use u8writer::U8Writer;
use gps::Gps;

type Display = memory_display::MemoryDisplay<stm32f4xx_hal::spi::Spi<stm32f4xx_hal::pac::SPI1, (stm32f4xx_hal::gpio::Pin<stm32f4xx_hal::gpio::Input<stm32f4xx_hal::gpio::Floating>, 'A', 5_u8>, stm32f4xx_hal::gpio::NoPin, stm32f4xx_hal::gpio::Pin<stm32f4xx_hal::gpio::Input<stm32f4xx_hal::gpio::Floating>, 'A', 7_u8>), stm32f4xx_hal::spi::TransferModeNormal>, stm32f4xx_hal::gpio::Pin<stm32f4xx_hal::gpio::Output<stm32f4xx_hal::gpio::PushPull>, 'A', 4_u8>, stm32f4xx_hal::timer::SysDelay, 12000_usize, 240_usize>;
type Serial = stm32f4xx_hal::serial::Serial<stm32f4xx_hal::pac::USART1, (stm32f4xx_hal::gpio::Pin<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::PushPull, 7_u8>, 'A', 9_u8>, stm32f4xx_hal::gpio::Pin<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::PushPull, 7_u8>, 'A', 10_u8>), u8>;
type Key = stm32f4xx_hal::gpio::gpioa::PA0<stm32f4xx_hal::gpio::Input<stm32f4xx_hal::gpio::PullUp>>;

#[app(device = stm32f4xx_hal::pac, peripherals = true)]
const APP: () = {
  struct Resources {
    display: Display,
    serial: Serial,
    gps: Gps,
    key: Key, 
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
      .use_hse(25.MHz())
      .sysclk(32.MHz())
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
    let key   = gpioa.pa0.into_pull_up_input();

    // configure the display driver
    let cs = gpioa.pa4.into_push_pull_output();
    let spi = cx.device.SPI1.spi(
        (gpioa.pa5, NoPin, gpioa.pa7),
        Mode {
          phase: Phase::CaptureOnFirstTransition,
          polarity: Polarity::IdleLow,
        },
        2.MHz(),
        &clocks,
    );

    let delay = cx.core.SYST.delay(&clocks);
    let display = memory_display::new_ls027b7dh01(spi, cs, delay);

    // Configure the serial port for GPS data
    let tx = gpioa.pa9.into_alternate();
    let rx = gpioa.pa10.into_alternate();

    let mut serial : Serial = cx.device.USART1.serial(
      (tx,rx), 
      9600.bps(), 
      &clocks
    ).unwrap();
    serial.listen(serial::Event::Rxne);

    init::LateResources {
      display,
      serial,
      gps: Gps::new(),
      key,
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

  #[idle(resources=[gps,display, key])]
  fn idle(mut cx: idle::Context) -> ! {
    let mut screens = layout::Screens::new();
    let mut debounce = debouncer::Debouncer::new(3);

    screens.render_initial(cx.resources.display).unwrap();
    cx.resources.display.refresh();

    loop {
      let key = cx.resources.key.is_high();
      match debounce.next(key) {
        Option::Some(debouncer::Transition::ToLow) => {
          screens.next_page(cx.resources.display).unwrap();
          cx.resources.display.refresh();
        }
        _ => {}
      }


      // Fetch the updated GGA and VTG values, if present
      let mut oogga: Option<Option<nmea0183::GGA>> = Option::None;
      let mut ovtg: Option<nmea0183::VTG> = Option::None;
      let mut speed_stats = gps::SpeedStats::new();
      
      cx.resources.gps.lock( |gps| {
        oogga = gps.take_gga();
        ovtg = gps.take_vtg();
        if ovtg.is_some() {
          speed_stats = gps.speed_stats();
        }
      });

      let mut updated = false;
      if let Some(ogga) = oogga {
        screens.update_gga(ogga);
        updated = true;
      }

      if let Some(vtg) = ovtg {
        screens.update_vtg(vtg, &speed_stats);
        updated = true;
      }

      if updated {
        screens.render_update(cx.resources.display).unwrap();
      }
      cx.resources.display.refresh();
    }
  }



  // RTIC requires that unused interrupts are declared in an extern block when
  // using software tasks; these free interrupts will be used to dispatch the
  // software tasks.
  extern "C" {
      fn USART3();
  }
};

