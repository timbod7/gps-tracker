use embedded_hal::blocking::delay;
use embedded_hal::blocking::spi;
use embedded_hal::digital::v2::OutputPin;

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{OriginDimensions, Point, Size};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::Pixel;

pub fn new_ls027b7dh01<SpiE, PinE, SPI, CS, DELAY>(
    spi: SPI,
    cs: CS,
    delay: DELAY,
) -> MemoryDisplay<SPI, CS, DELAY, 12000, 240>
where
    SPI: spi::Transfer<u8, Error = SpiE> + spi::Write<u8, Error = SpiE>,
    CS: OutputPin<Error = PinE>,
    DELAY: delay::DelayUs<u32>,
    SpiE: core::fmt::Debug,
    PinE: core::fmt::Debug,
{
    return MemoryDisplay::new(spi, cs, delay);
}

pub struct MemoryDisplay<SPI, CS, DELAY, const N: usize, const H: usize> {
    spi: SPI,
    cs: CS,
    delay: DELAY,
    vcom: bool,
    height: usize,
    widthpx: usize,
    widthb: usize,
    framebuf: [u8; N],
    dirty_lines: [bool; H],
    pending_clear: bool,
}

impl<SpiE, PinE, SPI, CS, DELAY, const N: usize, const H: usize> MemoryDisplay<SPI, CS, DELAY, N, H>
where
    SPI: spi::Transfer<u8, Error = SpiE> + spi::Write<u8, Error = SpiE>,
    CS: OutputPin<Error = PinE>,
    DELAY: delay::DelayUs<u32>,
    SpiE: core::fmt::Debug,
    PinE: core::fmt::Debug,
{
    pub fn new(spi: SPI, cs: CS, delay: DELAY) -> Self {
        let mut display = MemoryDisplay {
            spi,
            cs,
            delay,
            vcom: false,
            height: H,
            widthpx: N / H * 8,
            widthb: N / H,
            framebuf: [255; N],
            dirty_lines: [false; H],
            pending_clear: false,
        };
        display.cs.set_low().unwrap();
        display.delay.delay_us(3_u32);
        display.clear();
        display
    }

    fn clear(&mut self) {
        let mut buf: [u8; 2] = [0; 2];

        self.cs.set_high().unwrap();
        self.delay.delay_us(3_u32);

        buf[0] = 32 | self.toggle_vcom();
        buf[1] = 0;
        self.spi.write(&buf).unwrap();

        self.delay.delay_us(1_32);
        self.cs.set_low().unwrap();
        self.delay.delay_us(1_u32);
    }

    pub fn refresh(&mut self) {
        let mut needs_vcom_toggle = true;

        if self.pending_clear {
            self.clear();
            self.pending_clear = false;
            needs_vcom_toggle = false;
        }

        let mut y: usize = 0;
        let mut buf: [u8; 1] = [0; 1];

        while y < self.height {
            if !self.dirty_lines[y] {
                y += 1;
                continue;
            }

            self.enable_cs();

            // Write MODE
            buf[0] = 128 | self.toggle_vcom();
            self.spi.write(&buf).unwrap();

            while y < self.height && self.dirty_lines[y] {
                // Write LINE ADDR
                buf[0] = (y as u8 + 1).reverse_bits();
                self.spi.write(&buf).unwrap();

                // Write PIXELS
                let i = y * self.widthb;
                let pixels = &self.framebuf[i..i + self.widthb];
                self.spi.write(pixels).unwrap();

                // Write DUMMY BYTE
                buf[0] = 0;
                self.spi.write(&buf).unwrap();

                y += 1;
            }

            // Write DUMMY BYTE
            buf[0] = 0;
            self.spi.write(&buf).unwrap();

            self.disable_cs();

            needs_vcom_toggle = false;
        }

        if needs_vcom_toggle {
            self.enable_cs();
            buf[0] = self.toggle_vcom();
            self.spi.write(&buf).unwrap();
            buf[0] = 0;
            self.spi.write(&buf).unwrap();
            self.disable_cs();
        }
    }

    pub fn delay_us(&mut self, us: u32) {
        self.delay.delay_us(us);
    }

    fn enable_cs(&mut self) {
        self.cs.set_high().unwrap();
        self.delay.delay_us(3_u32);
    }

    fn disable_cs(&mut self) {
        self.delay.delay_us(1_u32);
        self.cs.set_low().unwrap();
        self.delay.delay_us(1_u32);
    }

    fn toggle_vcom(&mut self) -> u8 {
        self.vcom = !self.vcom;
        if self.vcom {
            64
        } else {
            0
        }
    }
}

impl<SPI, CS, DELAY, const W: usize, const H: usize> DrawTarget
    for MemoryDisplay<SPI, CS, DELAY, W, H>
{
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let bounds = Rectangle::new(Point::new(0, 0), self.size());

        for Pixel(coord, color) in pixels.into_iter() {
            if !bounds.contains(coord) {
                continue;
            }

            let x = coord.x as usize;
            let y = coord.y as usize;
            let byte_offset = y * self.widthb + (x / 8);
            let bit = 128u8.rotate_right((x % 8) as u32);

            if color == BinaryColor::Off {
                self.framebuf[byte_offset] |= bit;
            } else {
                self.framebuf[byte_offset] &= !bit;
            }
            self.dirty_lines[y] = true;
        }
        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area0: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let bounds = Rectangle::new(Point::new(0, 0), self.size());
        let area = area0.intersection(&bounds);
        if area.size == Size::zero() {
            return Ok(());
        }

        let x0 = area.top_left.x as usize;
        let x1 = x0 + (area.size.width as usize);
        let y0 = area.top_left.y as usize;
        let y1 = y0 + (area.size.height as usize);
        let mut cs = colors.into_iter();

        for y in y0..y1 {
            for x in x0..x1 {
                let byte_offset = y * self.widthb + (x / 8);
                let bit = 128u8.rotate_right((x % 8) as u32);
                let color = cs.next().unwrap();

                if color == BinaryColor::Off {
                    self.framebuf[byte_offset] |= bit;
                } else {
                    self.framebuf[byte_offset] &= !bit;
                }
            }
            self.dirty_lines[y] = true;
        }

        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        for b in self.framebuf.iter_mut() {
            *b = if color == BinaryColor::Off {
                0xff
            } else {
                0x00
            };
        }
        for e in self.dirty_lines.iter_mut() {
            *e = color == BinaryColor::On;
        }

        if color == BinaryColor::Off {
            self.pending_clear = true;
        }
        Ok(())
    }
}

impl<SPI, CS, DELAY, const N: usize, const H: usize> OriginDimensions
    for MemoryDisplay<SPI, CS, DELAY, N, H>
{
    fn size(&self) -> Size {
        let w: u32 = self.widthpx as u32;
        let h: u32 = self.height as u32;
        Size::new(w, h)
    }
}
