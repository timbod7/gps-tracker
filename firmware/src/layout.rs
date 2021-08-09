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

  pub fn write_speed<D>(&self, display: &mut D, speed: f32)-> Result<(), D::Error>
  where
    D: DrawTarget<Color = Rgb565>
  {
    let mut buf = [0u8; 20];
    let mut buf = U8Writer::new(&mut buf[..]);

    let mut cursor =  Layout::char_point(Point::new(4,4));

    buf.clear();
    write!(&mut buf, "{:2}", speed.trunc() ).unwrap();
    cursor = self.write_big_text(display, cursor, buf.as_str())?;
    cursor = self.write_big_dp(display, cursor)?;
    buf.clear();
    write!(&mut buf, "{:1}", (speed.fract() * 10.0).round()).unwrap();
    let _ = self.write_big_text(display, cursor, buf.as_str())?;

    Result::Ok(())
  }

  fn char_point(loc:Point) -> Point {
    Point{
      x: 4 + loc.x * CHAR_WIDTH, 
      y: 10 + loc.y * CHAR_HEIGHT
    }
  }
}