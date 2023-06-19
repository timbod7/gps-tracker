use crate::u8writer::U8Writer;
use crate::write_field;
use core::fmt::Write;
use core::str;
use embedded_graphics::primitives::Circle;
use embedded_graphics::{
    image::ImageRaw,
    mono_font::{
        mapping::StrGlyphMapping, DecorationDimensions, MonoFont, MonoTextStyle,
        MonoTextStyleBuilder,
    },
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text, TextStyle},
};

pub type DPixelColor = BinaryColor;
pub const WHITE: DPixelColor = BinaryColor::On;
pub const BLACK: DPixelColor = BinaryColor::Off;

const BIGNUMBER_FONT: MonoFont = MonoFont {
    image: ImageRaw::new_binary(include_bytes!("assets/bignumbers.raw"), 1200),
    glyph_mapping: &StrGlyphMapping::new("0123456789", 0),
    character_size: Size::new(120, 156),
    character_spacing: 0,
    baseline: 7,
    underline: DecorationDimensions::default_underline(40),
    strikethrough: DecorationDimensions::default_strikethrough(40),
};

const MEDNUMBER_FONT: MonoFont = MonoFont {
    image: ImageRaw::new_binary(include_bytes!("assets/mednumbers.raw"), 600),
    glyph_mapping: &StrGlyphMapping::new("0123456789", 0),
    character_size: Size::new(60, 78),
    character_spacing: 0,
    baseline: 7,
    underline: DecorationDimensions::default_underline(40),
    strikethrough: DecorationDimensions::default_strikethrough(40),
};

type MTextStyle = MonoTextStyle<'static, DPixelColor>;

pub struct Layout {
    pub char_18: MTextStyle,
    pub char_78: MTextStyle,
    pub char_156: MTextStyle,
    pub text_style: TextStyle,
    pub bg_fill_style: PrimitiveStyle<DPixelColor>,
    pub fg_fill_style: PrimitiveStyle<DPixelColor>,
}

impl Layout {
    pub fn new() -> Layout {
        let char_18 = MonoTextStyleBuilder::new()
            .font(&profont::PROFONT_18_POINT)
            .text_color(WHITE)
            .background_color(BLACK)
            .build();
        let char_78 = MonoTextStyleBuilder::new()
            .font(&MEDNUMBER_FONT)
            .text_color(WHITE)
            .background_color(BLACK)
            .build();
        let char_156 = MonoTextStyleBuilder::new()
            .font(&BIGNUMBER_FONT)
            .text_color(WHITE)
            .background_color(BLACK)
            .build();
        let text_style = TextStyle::with_baseline(Baseline::Top);
        let bg_fill_style = PrimitiveStyle::with_fill(BLACK);
        let fg_fill_style = PrimitiveStyleBuilder::new()
            .fill_color(WHITE)
            .stroke_color(WHITE)
            .stroke_width(2)
            .build();
        return Layout {
            char_18,
            char_78,
            char_156,
            text_style,
            bg_fill_style,
            fg_fill_style,
        };
    }

    pub fn clear<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        Rectangle::new(Point::new(0, 0), Size::new(480, 320))
            .into_styled(self.bg_fill_style)
            .draw(display)?;
        Result::Ok(())
    }

    pub fn write_str<D>(
        &self,
        char_style: &MTextStyle,
        display: &mut D,
        loc: Point,
        content: &str,
    ) -> Result<Point, D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        Text::with_text_style(content, loc, *char_style, self.text_style).draw(display)
    }

    pub fn write_char<D>(
        &self,
        char_style: &MTextStyle,
        display: &mut D,
        loc: Point,
        c: char,
    ) -> Result<Point, D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        let mut buf: [u8; 4] = [0; 4];
        let mut w = U8Writer::new(&mut buf);
        w.write_char(c).unwrap();
        Text::with_text_style(w.as_str(), loc, *char_style, self.text_style).draw(display)
    }

    pub fn write_kerned_str<D>(
        &self,
        char_style: &MTextStyle,
        display: &mut D,
        loc: Point,
        content: &str,
    ) -> Result<Point, D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        // Decimal points have their own kerning, so treat them specially
        let mut remaining = content;
        let mut cursor = loc;
        loop {
            let d = remaining.find('.');
            let s = &remaining[0..d.unwrap_or(remaining.len())];
            cursor =
                Text::with_text_style(s, cursor, *char_style, self.text_style).draw(display)?;
            if let Some(idp) = d {
                cursor = self.write_kerned_dp(char_style, display, cursor)?;
                remaining = &remaining[idp + 1..];
            } else {
                break Ok(cursor);
            }
        }
    }

    pub fn write_kerned_dp<D>(
        &self,
        char_style: &MTextStyle,
        display: &mut D,
        loc: Point,
    ) -> Result<Point, D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        let char_height = char_style.font.character_size.height as i32;
        let diam = char_height / 7;
        let kern = 0;
        let topleft = loc + Point::new(kern, char_height - diam - 4);
        Circle::new(topleft, diam as u32)
            .into_styled(self.fg_fill_style)
            .draw(display)?;
        Result::Ok(loc + Point::new(kern * 2 + diam, 0))
    }

    pub fn render_field<D, const N: usize>(
        &self,
        char_style: &MTextStyle,
        display: &mut D,
        cursor0: Point,
        field: &mut DisplayField<N>,
    ) -> Result<Point, D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        let mut cursor = cursor0;
        let char_width = char_style.font.character_size.width as i32;
        let left = Point::new(char_width, 0);
        for i in 0..field.buf.len() {
            if let Some(c) = field.getdirtychar(i) {
                self.write_str(char_style, display, cursor, c)?;
            }
            cursor = cursor + left;
        }
        field.clear_dirty();
        Result::Ok(cursor)
    }

    pub fn char_point(&self, char_style: &MTextStyle, x: i32, y: i32) -> Point {
        let char_width = char_style.font.character_size.width as i32;
        let char_height = char_style.font.character_size.height as i32;
        Point {
            x: 4 + x * char_width,
            y: 10 + y * char_height,
        }
    }
}

/// A fixed width text display field that keeps track of which
/// characters have been updated, for efficient updates.
pub struct DisplayField<const W: usize> {
    buf: [u8; W],
    dirty: [bool; W], // TODO: implement as a bitmap
}

impl<const W: usize> DisplayField<W> {
    pub fn new() -> Self {
        DisplayField {
            buf: [' ' as u8; W],
            dirty: [true; W],
        }
    }
    pub fn from_str(s: &str) -> Self {
        let mut f = DisplayField::new();
        write_field!(f, "{}", s).unwrap();
        f
    }

    pub fn tmpbuf(&self) -> [u8; W] {
        [0; W]
    }

    pub fn getdirtychar(&self, i: usize) -> Option<&str> {
        if self.dirty[i] {
            Option::Some(str::from_utf8(&self.buf[i..i + 1]).unwrap())
        } else {
            Option::None
        }
    }

    pub fn update_from(&mut self, buf: &[u8; W]) {
        for i in 0..self.buf.len() {
            if self.buf[i] != buf[i] {
                self.buf[i] = buf[i];
                self.dirty[i] = true;
            }
        }
    }

    pub fn clear(&mut self) {
        self.update_from(&[' ' as u8; W]);
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = [false; W];
    }
}

#[macro_export]
macro_rules! write_field {
  ($displayfield:expr, $($arg:tt)*) => {
    {
      let mut buf = $displayfield.tmpbuf();
      let mut u8w = U8Writer::new(&mut buf);
      let r = u8w.write_fmt(core::format_args!($($arg)*));
      u8w.fill(' ' as u8);
      $displayfield.update_from(&buf);
      r
    }
  }
}

#[macro_export]
macro_rules! write_field0 {
  ($buf:expr, $($arg:tt)*) => {
    {
      let mut buf = $buf;
      let mut u8w = U8Writer::new(&mut buf);
      let r = u8w.write_fmt(core::format_args!($($arg)*));
      from_utf8(&buf[0..u8w.len()]).unwrap()
    }
  }
}
