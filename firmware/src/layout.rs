use embedded_graphics::primitives::Rectangle;
use crate::PrimitiveStyle;
use embedded_graphics::{
  image::ImageRaw,
  mono_font::{mapping::StrGlyphMapping, DecorationDimensions, MonoFont, MonoTextStyle,MonoTextStyleBuilder},
  pixelcolor::Rgb565,
  prelude::*,
  text::{Baseline, Text, TextStyle},
};

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
    return Layout {
      char_style,
      big_char_style,
      text_style,
      bg_fill_style,
    }
  }

  pub fn clear<D>(&self, display: &mut D) -> Result<(), D::Error>
  where
    D: DrawTarget<Color = Rgb565>
  {
    Rectangle::new(Point::new(0,0), Size::new(480, 320))
    .into_styled(self.bg_fill_style)
    .draw(display)
  }

  pub fn write_big_text<D>(&self, display: &mut D, cloc: Point, content: &str) -> Result<Point, D::Error>
  where
    D: DrawTarget<Color = Rgb565>
  {
    let loc  = Layout::char_point(cloc);
    Text::with_text_style(content, loc, self.big_char_style, self.text_style)
      .draw(display)
  }

  pub fn write_field<D>(&self, display: &mut D, cloc: Point, width: i32, content: &str) -> Result<(), D::Error>
  where
    D: DrawTarget<Color = Rgb565>
  {
    let loc  = Layout::char_point(cloc);
    let p = Text::with_text_style(content, loc, self.char_style, self.text_style)
      .draw(display)?;

    // TODO FILL EXTRA BACKGROUND
    Result::Ok(())
  }

  fn char_point(loc:Point) -> Point {
    Point{
      x: 4 + loc.x * CHAR_WIDTH, 
      y: 10 + loc.y * CHAR_HEIGHT
    }
  }
}