use embedded_graphics::{
  image::ImageRaw,
  mono_font::{mapping::StrGlyphMapping, DecorationDimensions, MonoFont, MonoTextStyle},
  pixelcolor::BinaryColor,
  prelude::*,
  text::{Alignment, Baseline, Text, TextStyleBuilder},
};

pub const BIGNUMBER_FONT: MonoFont = MonoFont {
  image: ImageRaw::new_binary(include_bytes!("assets/bignumbers.raw"), 600),
  glyph_mapping: &StrGlyphMapping::new("0123456789", 0),
  character_size: Size::new(60, 80),
  character_spacing: 0,
  baseline: 7,
  underline: DecorationDimensions::default_underline(40),
  strikethrough: DecorationDimensions::default_strikethrough(40),
};


pub const SEVEN_SEGMENT_FONT: MonoFont = MonoFont {
  image: ImageRaw::new_binary(include_bytes!("assets/seven-segment-font.raw"), 224),
  glyph_mapping: &StrGlyphMapping::new("0123456789", 0),
  character_size: Size::new(22, 40),
  character_spacing: 4,
  baseline: 14,
  underline: DecorationDimensions::default_underline(40),
  strikethrough: DecorationDimensions::default_strikethrough(40),
};