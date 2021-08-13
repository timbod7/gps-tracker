use embedded_graphics::primitives::Circle;
use crate::U8Writer;
use embedded_graphics::{
  image::ImageRaw,
  mono_font::{mapping::StrGlyphMapping, DecorationDimensions, MonoFont, MonoTextStyle,MonoTextStyleBuilder},
  pixelcolor::Rgb565,
  prelude::*,
  text::{Baseline, Text, TextStyle},
  primitives::{Rectangle, PrimitiveStyle},
};
use core::fmt::{Write};
use core::str;
use micromath::F32Ext;


const BIGNUMBER_FONT: MonoFont = MonoFont {
  image: ImageRaw::new_binary(include_bytes!("assets/bignumbers.raw"), 660),
  glyph_mapping: &StrGlyphMapping::new("0123456789.", 0),
  character_size: Size::new(60, 80),
  character_spacing: 0,
  baseline: 7,
  underline: DecorationDimensions::default_underline(40),
  strikethrough: DecorationDimensions::default_strikethrough(40),
};

pub struct Layout {
  char_style: MonoTextStyle<'static, Rgb565>,
  big_char_style: MonoTextStyle<'static, Rgb565>,
  text_style: TextStyle,
  bg_fill_style: PrimitiveStyle<Rgb565>,
  fg_fill_style: PrimitiveStyle<Rgb565>,
}

const CHAR_WIDTH: i32 = 12;
const CHAR_HEIGHT: i32 = 22;
const BIG_CHAR_WIDTH: i32 = 60;

impl Layout {
  pub fn new() -> Layout {
    let char_style = MonoTextStyleBuilder::new()
      .font(&profont::PROFONT_18_POINT)
      .text_color(Rgb565::WHITE)
      .background_color(Rgb565::BLACK)
      .build();
    let big_char_style = MonoTextStyleBuilder::new()
      .font(&BIGNUMBER_FONT)
      .text_color(Rgb565::WHITE)
      .background_color(Rgb565::BLACK)
      .build();
    let text_style = TextStyle::with_baseline(Baseline::Top);
    let bg_fill_style = PrimitiveStyle::with_fill(Rgb565::BLACK);
    let fg_fill_style = PrimitiveStyle::with_fill(Rgb565::WHITE);
    return Layout {
      char_style,
      big_char_style,
      text_style,
      bg_fill_style,
      fg_fill_style,
    }
  }

  pub fn clear<D>(&self, display: &mut D) -> Result<(), D::Error>
  where
    D: DrawTarget<Color = Rgb565>
  {
    Rectangle::new(Point::new(0,0), Size::new(480, 320))
    .into_styled(self.bg_fill_style)
    .draw(display)?;
    Result::Ok(())
  }

  pub fn write_big_text<D>(&self, display: &mut D, loc: Point, content: &str) -> Result<Point, D::Error>
  where
    D: DrawTarget<Color = Rgb565>
  {
    Text::with_text_style(content, loc, self.big_char_style, self.text_style)
      .draw(display)
  }
  
  pub fn write_big_dp<D>(&self, display: &mut D, loc: Point) -> Result<Point, D::Error>
  where
    D: DrawTarget<Color = Rgb565>
  {
    let diam = 13;
    let kern = 2;
    let topleft = loc + Point::new(kern, 80-diam-4);
    Circle::new(topleft, diam as u32)
      .into_styled(self.fg_fill_style)
      .draw(display)?;
    Result::Ok(loc + Point::new(kern * 2 + diam, 0))
  }

  pub fn write_field<D>(&self, display: &mut D, cloc: Point, width: i32, content: &str) -> Result<(), D::Error>
  where
    D: DrawTarget<Color = Rgb565>
  {
    let loc  = Layout::char_point(cloc);
    let _ = Text::with_text_style(content, loc, self.char_style, self.text_style)
      .draw(display)?;

    // TODO FILL EXTRA BACKGROUND
    Result::Ok(())
  }

  pub fn write_speed<D>(&self, display: &mut D, speed: &mut DisplayField<3>)-> Result<(), D::Error>
  where
    D: DrawTarget<Color = Rgb565>
  {
    let mut cursor =  Layout::char_point(Point::new(4,4));
    let nextc = Point::new(BIG_CHAR_WIDTH, 0);

    if let Some(c) = speed.getdirtychar(0) {
      cursor = self.write_big_text(display, cursor, c)?
    } else {
      cursor = cursor + nextc;
    }
    if let Some(c) = speed.getdirtychar(1) {
      cursor = self.write_big_text(display, cursor, c)?
    } else {
      cursor = cursor + nextc;
    }
    cursor = self.write_big_dp(display, cursor)?;
    if let Some(c) = speed.getdirtychar(2) {
      cursor = self.write_big_text(display, cursor, c)?
    } else {
      cursor = cursor + nextc;
    }
    speed.clear_dirty();
    Result::Ok(())
  }

  fn char_point(loc:Point) -> Point {
    Point{
      x: 4 + loc.x * CHAR_WIDTH, 
      y: 10 + loc.y * CHAR_HEIGHT
    }
  }
}


/// A fixed width text display field that keeps track of which
/// characters have been updated, for efficient updates.
pub struct DisplayField<const W: usize> {
  buf: [u8; W],
  dirty: [bool; W],    // TODO: implement as a bitmap
}

impl<const W: usize> DisplayField<W> {
  pub fn new()-> Self {
    DisplayField {
      buf: [0; W],
      dirty: [false; W],
    }
  }

  pub fn tmpbuf(&self) -> [u8; W] {
    [0; W]
  }

  pub fn getdirtychar(&self, i: usize) -> Option<&str> {
    if self.dirty[i] {
      Option::Some(str::from_utf8(&self.buf[i..i+1]).unwrap())
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


// pub fn test() {
//   let mut field: DisplayField<6> = DisplayField::new();
//  write_field!(field, "Long: {:.6}", "14.22").unwrap();
// }