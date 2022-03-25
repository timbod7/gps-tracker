#![no_main]
#![no_std]
#[allow(dead_code)]

mod u8writer;
mod layout;
mod gps;
mod debouncer;
mod memory_display;

// set the panic handler
extern crate panic_rtt_target;


#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [EXTI0])]
mod app {

use embedded_hal::spi::{Mode, Phase, Polarity};
use stm32f4xx_hal::{
  gpio::{NoPin},
  adc,
  prelude::*,
  serial
};

use rtt_target::{rtt_init_print, rprintln};

use nb::block;

use crate::gps;
use crate::gps::Gps;
use crate::layout;
use crate::debouncer;
use crate::memory_display;

type Display = crate::memory_display::MemoryDisplay<stm32f4xx_hal::spi::Spi<stm32f4xx_hal::pac::SPI1, (stm32f4xx_hal::gpio::Pin<stm32f4xx_hal::gpio::Input<stm32f4xx_hal::gpio::Floating>, 'A', 5_u8>, stm32f4xx_hal::gpio::NoPin, stm32f4xx_hal::gpio::Pin<stm32f4xx_hal::gpio::Input<stm32f4xx_hal::gpio::Floating>, 'A', 7_u8>), stm32f4xx_hal::spi::TransferModeNormal>, stm32f4xx_hal::gpio::Pin<stm32f4xx_hal::gpio::Output<stm32f4xx_hal::gpio::PushPull>, 'A', 4_u8>, stm32f4xx_hal::timer::SysDelay, 12000_usize, 240_usize>;
type Serial = stm32f4xx_hal::serial::Serial<stm32f4xx_hal::pac::USART1, (stm32f4xx_hal::gpio::Pin<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::PushPull, 7_u8>, 'A', 9_u8>, stm32f4xx_hal::gpio::Pin<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::PushPull, 7_u8>, 'A', 10_u8>), u8>;
type Key = stm32f4xx_hal::gpio::gpioa::PA0<stm32f4xx_hal::gpio::Input<stm32f4xx_hal::gpio::PullUp>>;
type Adc = stm32f4xx_hal::adc::Adc<stm32f4xx_hal::pac::ADC1>;
type Vin = stm32f4xx_hal::gpio::gpioa::PA1<stm32f4xx_hal::gpio::Analog>;


  #[shared]
  struct Shared {
    gps: Gps,
  }

  #[local]
  struct Local {
    serial: Serial,
    display: Display,
    key: Key,
    adc: Adc,
    vin: Vin,
  }

  #[init()]
  fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
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
    let vin = gpioa.pa1.into_analog();

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

    // Configure the ADC for battery voltage
    let adc_config = adc::config::AdcConfig::default();
    let adc = adc::Adc::adc1(cx.device.ADC1, true, adc_config);

    let shared = Shared {
      gps: Gps::new(),
    };

    let local = Local {
      serial,
      display,
      key,
      adc,
      vin,
    };

    (shared, local, init::Monotonics())

  }

  #[task(binds = USART1, shared=[gps], local=[serial])]
  fn usart1(mut cx: usart1::Context) {
    let ereceived: Result<u8,serial::Error> = block!(cx.local.serial.read());
    match ereceived {
      Result::Err(_e) => {
        rprintln!("usart1: error");
        cx.shared.gps.lock( |gps| gps.parse_clear());
      },
      Result::Ok(received) => {
        cx.shared.gps.lock( |gps| gps.parse_u8(received));
      },
    }
  }

  #[idle(shared=[gps], local=[key,display,adc,vin])]
  fn idle(mut cx: idle::Context) -> ! {
    let mut screens = layout::Screens::new();
    let mut debounce = debouncer::Debouncer::new(3);

    screens.render_initial(cx.local.display).unwrap();
    cx.local.display.refresh();

    loop {
      let key = cx.local.key.is_high();
      match debounce.next(key) {
        Option::Some(debouncer::Transition::ToLow) => {
          screens.next_page(cx.local.display).unwrap();
          cx.local.display.refresh();
        }
        _ => {}
      }

      // Fetch the updated GGA and VTG values, if present
      let mut oogga: Option<Option<nmea0183::GGA>> = Option::None;
      let mut ovtg: Option<nmea0183::VTG> = Option::None;
      let mut speed_stats = gps::SpeedStats::new();
      
      cx.shared.gps.lock( |gps| {
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

      
      let sample = cx.local.adc.convert(cx.local.vin, adc::config::SampleTime::Cycles_480);
      let vbat = cx.local.adc.sample_to_millivolts(sample) * 2;
      screens.update_vbat(vbat);
      updated = true;

      if updated {
        screens.render_update(cx.local.display).unwrap();
      }
      cx.local.display.refresh();
    }
  }
}

