use nmea0183::VTG;
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
use nmea0183::GGA;
use crate::write_field;


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

pub const CHAR_WIDTH: i32 = 12;
pub const CHAR_HEIGHT: i32 = 22;
pub const BIG_CHAR_WIDTH: i32 = 60;

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

  pub fn write_text<D>(&self, display: &mut D, loc: Point, content: &str) -> Result<Point, D::Error>
  where
    D: DrawTarget<Color = Rgb565>
  {
    Text::with_text_style(content, loc, self.char_style, self.text_style)
      .draw(display)
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

  pub fn render_field<D, const N:usize>(&self, display: &mut D, cursor0: Point, field: &mut DisplayField<N>) -> Result<Point, D::Error>
  where
    D: DrawTarget<Color = Rgb565>
  {
    let mut cursor = cursor0;
    let left = Point::new(CHAR_WIDTH, 0);
    for i in 0..field.buf.len() {
      if let Some(c) = field.getdirtychar(i) {
        self.write_text(display, cursor, c)?;
      }
      cursor = cursor + left;

    }
    field.clear_dirty();
    Result::Ok(cursor)
  }

  pub fn char_point(&self, x: i32, y: i32) -> Point {
    Point{
      x: 4 + x * CHAR_WIDTH, 
      y: 10 + y * CHAR_HEIGHT
    }
  }
}

pub struct Screen1 {
  layout: Layout,
  speed_field : DisplayField<3>,  
  sats_field  : DisplayField<8>,  
  lat_field   : DisplayField<18>,  
  lng_field   : DisplayField<18>,  
}

impl Screen1 {
  pub fn new() -> Self {
    Screen1 {
      layout : Layout:: new(),
      speed_field: DisplayField::new(),
      sats_field: DisplayField::new(),
      lat_field: DisplayField::new(),
      lng_field: DisplayField::new(),
    }
  }

  pub fn render_initial<D>(&mut self,  display: &mut D )-> Result<(), D::Error>
  where
  D: DrawTarget<Color = Rgb565>
  {
    write_field!(self.speed_field, "000");
    self.layout.clear(display)?;
    self.layout.write_text(display, self.layout.char_point(21, 4), "kt")?;
    Result::Ok(())
  }


  pub fn render_update<D>(&mut self,  display: &mut D )-> Result<(), D::Error>
  where
  D: DrawTarget<Color = Rgb565>
  {
    let mut cursor = Point::new(0,0);
    let down = Point::new(0,CHAR_HEIGHT);
    self.layout.render_field(display, cursor, &mut self.sats_field)?;
    cursor = cursor + down;
    self.layout.render_field(display, cursor, &mut self.lat_field)?;
    cursor = cursor + down;
    self.layout.render_field(display, cursor, &mut self.lng_field)?;

    self.render_speed(display)?;

    Result::Ok(())
  }

  fn render_speed<D>(&mut self, display: &mut D)-> Result<(), D::Error>
  where
    D: DrawTarget<Color = Rgb565>
  {
    let mut cursor =  self.layout.char_point(4, 4);
    let nextc = Point::new(BIG_CHAR_WIDTH, 0);

    if let Some(c) = self.speed_field.getdirtychar(0) {
      cursor = self.layout.write_big_text(display, cursor, c)?
    } else {
      cursor = cursor + nextc;
    }
    if let Some(c) = self.speed_field.getdirtychar(1) {
      cursor = self.layout.write_big_text(display, cursor, c)?
    } else {
      cursor = cursor + nextc;
    }
    cursor = self.layout.write_big_dp(display, cursor)?;
    if let Some(c) = self.speed_field.getdirtychar(2) {
      cursor = self.layout.write_big_text(display, cursor, c)?
    } else {
      cursor = cursor + nextc;
    }
    self.speed_field.clear_dirty();
    Result::Ok(())
  }


  pub fn update_gga(&mut self, ogga: Option<GGA>) {
    match ogga {
      Option::None => {
        write_field!(self.sats_field, "Sats: 0");
        self.lat_field.clear();
        self.lng_field.clear();
      },
      Option::Some(gga) => {
        write_field!(self.sats_field, "Sats: {:2}", gga.sat_in_use);
        write_field!(self.lat_field, "Lat: {:12.6}", gga.latitude.as_f64()).unwrap();
        write_field!(self.lng_field, "Lng: {:12.6}", gga.longitude.as_f64()).unwrap();
      }
    }
  }

  pub fn update_vtg(&mut self, vtg: VTG) {
    write_field!(self.speed_field, "{:3}", (vtg.speed.as_knots() * 10.0).round() as u32);
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
      buf: [' ' as u8; W],
      dirty: [true; W],
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
