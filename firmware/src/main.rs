#![no_main]
#![no_std]

// set the panic handler
extern crate panic_halt;

use cortex_m_rt::{entry};
use stm32f1xx_hal::{prelude::*, stm32, spi, delay::Delay};
use ili9341::{Ili9341, Orientation};

use embedded_graphics::{
  mono_font::MonoTextStyle,
  pixelcolor::Rgb565,
  prelude::*,
  text::{Baseline, Text, TextStyle},
};

use display_interface_spi::SPIInterface;


#[entry]
fn main() -> ! {
    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = stm32::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.constrain();
    let mut flash = dp.FLASH.constrain();

    let clocks = rcc
        .cfgr
        .use_hse(8.mhz())
        .sysclk(32.mhz())
        .freeze(&mut flash.acr);

    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);

    // configure the gpio ports
    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);

    // configure the display driver
    let cs    = gpioa.pa2.into_push_pull_output(&mut gpioa.crl);
    let dc    = gpioa.pa3.into_push_pull_output(&mut gpioa.crl);
    let reset = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);
    let sck   = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
    let miso  = gpioa.pa6;
    let mosi  = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);
    let spi   = spi::Spi::spi1(
        dp.SPI1,
        (sck, miso, mosi),
        &mut afio.mapr,
        ili9341::SPI_MODE,
        16.mhz(),
        clocks,
        &mut rcc.apb2,
    );

    let mut delay = Delay::new(cp.SYST, clocks);
    let mut display = Ili9341::new(
      SPIInterface::new(spi, dc, cs),
      reset,
      &mut delay,
      Orientation::Landscape,
      ili9341::DisplaySize320x480,
      ).unwrap();

    let character_style = MonoTextStyle::new(&profont::PROFONT_24_POINT, Rgb565::WHITE);
    let text_style = TextStyle::with_baseline(Baseline::Top);

    let test_text = "Hello world!";

    Text::with_text_style(test_text, Point::zero(), character_style, text_style)
        .draw(&mut display).unwrap();


    loop {
        cortex_m::asm::wfi();
    }
}
