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

use core::alloc::Layout;

use embedded_hal::spi::{Mode, Phase, Polarity};
use stm32f4xx_hal::{
  gpio::{NoPin},
  adc,
  prelude::*,
  serial
};

use dwt_systick_monotonic::DwtSystick;

use rtt_target::{rtt_init_print, rprintln};

use nb::block;

use crate::gps::Gps;
use crate::layout;
use crate::debouncer;
use crate::memory_display;

type Display = crate::memory_display::MemoryDisplay<stm32f4xx_hal::spi::Spi<stm32f4xx_hal::pac::SPI1, (stm32f4xx_hal::gpio::Pin<stm32f4xx_hal::gpio::Input<stm32f4xx_hal::gpio::Floating>, 'A', 5_u8>, stm32f4xx_hal::gpio::NoPin, stm32f4xx_hal::gpio::Pin<stm32f4xx_hal::gpio::Input<stm32f4xx_hal::gpio::Floating>, 'A', 7_u8>), stm32f4xx_hal::spi::TransferModeNormal>, stm32f4xx_hal::gpio::Pin<stm32f4xx_hal::gpio::Output<stm32f4xx_hal::gpio::PushPull>, 'A', 4_u8>, stm32f4xx_hal::timer::Delay<stm32f4xx_hal::pac::TIM5, 1000000_u32>, 12000_usize, 240_usize>;
type Serial = stm32f4xx_hal::serial::Serial<stm32f4xx_hal::pac::USART1, (stm32f4xx_hal::gpio::Pin<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::PushPull, 7_u8>, 'A', 9_u8>, stm32f4xx_hal::gpio::Pin<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::PushPull, 7_u8>, 'A', 10_u8>), u8>;
type Key = stm32f4xx_hal::gpio::gpioa::PA0<stm32f4xx_hal::gpio::Input<stm32f4xx_hal::gpio::PullUp>>;
type Adc = stm32f4xx_hal::adc::Adc<stm32f4xx_hal::pac::ADC1>;
type Vin = stm32f4xx_hal::gpio::gpioa::PA1<stm32f4xx_hal::gpio::Analog>;
type Led = stm32f4xx_hal::gpio::gpioc::PC13<stm32f4xx_hal::gpio::Output<stm32f4xx_hal::gpio::PushPull>>;

const MONO_HZ: u32 = 84_000_000; // 8 MHz

#[monotonic(binds = SysTick, default = true)]
type MyMono = DwtSystick<MONO_HZ>;
type MyDuration = <dwt_systick_monotonic::DwtSystick<MONO_HZ> as rtic::Monotonic>::Duration;

  #[shared]
  struct Shared {
    gps: Gps,
    vbat_mv: Option<u16>,
  }

  #[local]
  struct Local {
    serial: Serial,
    display: Display,
    key: Key,
    adc: Adc,
    vbatin: Vin,
    led: Led,
  }

  #[init()]
  fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
    rtt_init_print!();
    rprintln!("init: START");

    // Take ownership over the raw flash and rcc devices and convert them into the corresponding
    // HAL structs
    let rcc = cx.device.RCC.constrain();

    let clocks = rcc
      .cfgr
      .use_hse(25.MHz())
      .sysclk(32.MHz())
      .freeze();

    let mut dcb = cx.core.DCB;
    let dwt = cx.core.DWT;
    let systick = cx.core.SYST;
    let mono: MyMono = DwtSystick::new(&mut dcb, dwt, systick, MONO_HZ);

    // Enable debugging in sleep modes so that stlink stays alive during wfi etc._
    // Remove this if/when power consumption is an issue.
    cx.device.DBGMCU.cr.write( |w| w
      .dbg_sleep().set_bit()
      .dbg_stop().set_bit()
      .dbg_standby().set_bit()
    );

    // configure the gpio ports
    let gpioa = cx.device.GPIOA.split();
    let gpioc = cx.device.GPIOC.split();
    let key   = gpioa.pa0.into_pull_up_input();
    let vbatin   = gpioa.pa1.into_analog();
    let mut led   = gpioc.pc13.into_push_pull_output();


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


    let delay = cx.device.TIM5.delay_us(&clocks);
    let mut display = memory_display::new_ls027b7dh01(spi, cs, delay);
    let mut layout  = crate::layout::Layout::new();
    layout.write_text(&mut display, layout.char_point(0,0), "booting...").unwrap();
    display.refresh();

    // Why is this nessary after power up
    cortex_m::asm::delay(16_000_000);

    // Configure the serial port for GPS data
    let tx = gpioa.pa9.into_alternate();
    let rx = gpioa.pa10.into_alternate();

    let mut serial : Serial = cx.device.USART1.serial(
      (tx,rx), 
      9600.bps(), 
      &clocks
    ).unwrap();
    let mut gps = Gps::new();

    rprintln!("init: gps");

    gps.init(&mut serial);
    led.set_low();
    serial.listen(serial::Event::Rxne);

    // Configure the ADC for battery voltage
    let adc_config = adc::config::AdcConfig::default();
    let adc = adc::Adc::adc1(cx.device.ADC1, true, adc_config);

    // Periodically read the battery voltage
    read_batv::spawn_after(250.millis()).unwrap();

    let shared = Shared {
      gps,
      vbat_mv: Some(0),
    };

    layout.clear(&mut display).unwrap();

    let local = Local {
      serial,
      display,
      key,
      adc,
      vbatin,
      led,
    };

    rprintln!("init: DONE");
    (shared, local, init::Monotonics(mono))
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

  #[idle(shared=[gps, vbat_mv], local=[key,display])]
  fn idle(mut cx: idle::Context) -> ! {
    rprintln!("idle0: START");
    let mut screens = layout::Screens::new();
    let mut debounce = debouncer::Debouncer::new(3);

    screens.render(cx.local.display).unwrap();
    screens.update_vbat(0);

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

      // Fetch the updated gps values, if present
      
      let ogps = cx.shared.gps.lock( |gps| {
        gps.take()
      });

      let mut updated = false;
      if let Some(gps) = ogps {
        screens.update_gps(&gps);
        updated = true;
      }

      let ovbat_mv = cx.shared.vbat_mv.lock( |vbat_mv| {vbat_mv.take()} );
      if let Some(vbat_mv) = ovbat_mv {
        screens.update_vbat(vbat_mv);
        updated = true;
      }

      if updated {
        screens.render(cx.local.display).unwrap();
      }
      cx.local.display.refresh();

    }
  }

  #[task(local=[adc,vbatin,led], shared=[vbat_mv])]
  fn read_batv(mut cx:  read_batv::Context) {
    rprintln!("read_batv");

    // Read the pin voltage, and write it to the shared variable
    let sample = cx.local.adc.convert(cx.local.vbatin, adc::config::SampleTime::Cycles_480);
    let mv = cx.local.adc.sample_to_millivolts(sample) * 2;
    cx.shared.vbat_mv.lock( |vbat_mv| *vbat_mv = Some(mv) );

    cx.local.led.toggle();
    read_batv::spawn_after(250.millis()).unwrap();
  }  
}


