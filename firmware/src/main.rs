#![no_main]
#![no_std]

use rtt_target::{rtt_init_print, rprintln};

// set the panic handler
extern crate panic_rtt_target;

use stm32f4xx_hal::{prelude::*, spi, delay::Delay};

use rtic::app;

use ili9341::{Ili9341, Orientation};
use embedded_graphics::{
  mono_font::MonoTextStyle,
  pixelcolor::Rgb565,
  prelude::*,
  text::{Baseline, Text, TextStyle},
};
use display_interface_spi::SPIInterface;

type Display = ili9341::Ili9341<display_interface_spi::SPIInterface<stm32f4xx_hal::spi::Spi<stm32f4xx_hal::stm32::SPI1, (stm32f4xx_hal::gpio::gpioa::PA5<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF5>>, stm32f4xx_hal::gpio::gpioa::PA6<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF5>>, stm32f4xx_hal::gpio::gpioa::PA7<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF5>>)>, stm32f4xx_hal::gpio::gpioa::PA3<stm32f4xx_hal::gpio::Output<stm32f4xx_hal::gpio::PushPull>>, stm32f4xx_hal::gpio::gpioa::PA2<stm32f4xx_hal::gpio::Output<stm32f4xx_hal::gpio::PushPull>>>, stm32f4xx_hal::gpio::gpioa::PA4<stm32f4xx_hal::gpio::Output<stm32f4xx_hal::gpio::PushPull>>>;

#[app(device = stm32f4xx_hal::pac, peripherals = true)]
const APP: () = {
  struct Resources {
    display: Display
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
    let mut display = Ili9341::new(
      SPIInterface::new(spi, dc, cs),
      reset,
      &mut delay,
      Orientation::Landscape,
      ili9341::DisplaySize320x480,
      ).unwrap();

    rprintln!("Starting init4");

    let character_style = MonoTextStyle::new(&profont::PROFONT_24_POINT, Rgb565::WHITE);
    let text_style = TextStyle::with_baseline(Baseline::Top);

    let test_text = "Hello world!";

    rprintln!("Starting init5");


    Text::with_text_style(test_text, Point::zero(), character_style, text_style)
        .draw(&mut display).unwrap();

        rprintln!("Starting init5");

    init::LateResources {
      display,
    }

  }

  #[idle]
  fn idle(_cx: idle::Context) -> ! {
    rprintln!("Starting idle");
    loop {
      cortex_m::asm::wfi();
    }
  }

  // RTIC requires that unused interrupts are declared in an extern block when
  // using software tasks; these free interrupts will be used to dispatch the
  // software tasks.
  extern "C" {
      fn USART1();
  }
};
