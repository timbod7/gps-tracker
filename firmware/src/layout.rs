use nmea0183::VTG;
use embedded_graphics::primitives::Circle;
use crate::U8Writer;
use embedded_graphics::{
  image::ImageRaw,
  mono_font::{mapping::StrGlyphMapping, DecorationDimensions, MonoFont, MonoTextStyle,MonoTextStyleBuilder},
  pixelcolor::BinaryColor,
  prelude::*,
  text::{Baseline, Text, TextStyle},
  primitives::{Rectangle, PrimitiveStyle},
};
use core::fmt::{Write};
use core::str;
use micromath::F32Ext;
use nmea0183::{GGA};
use crate::write_field;
use crate::gps::SpeedStats;

type DPixelColor = BinaryColor;
const WHITE: DPixelColor = BinaryColor::On;
const BLACK: DPixelColor =  BinaryColor::Off;


const BIGNUMBER_FONT: MonoFont = MonoFont {
  image: ImageRaw::new_binary(include_bytes!("assets/bignumbers.raw"), 1200),
  glyph_mapping: &StrGlyphMapping::new("0123456789", 0),
  character_size: Size::new(120, 156),
  character_spacing: 0,
  baseline: 7,
  underline: DecorationDimensions::default_underline(40),
  strikethrough: DecorationDimensions::default_strikethrough(40),
};

pub struct Layout {
  char_style: MonoTextStyle<'static, DPixelColor>,
  big_char_style: MonoTextStyle<'static, DPixelColor>,
  text_style: TextStyle,
  bg_fill_style: PrimitiveStyle<DPixelColor>,
  fg_fill_style: PrimitiveStyle<DPixelColor>,
}

pub const CHAR_WIDTH: i32 = 12;
pub const CHAR_HEIGHT: i32 = 22;
pub const BIG_CHAR_WIDTH: i32 = BIGNUMBER_FONT.character_size.width as i32;
pub const BIG_CHAR_HEIGHT: i32 = BIGNUMBER_FONT.character_size.height as i32;

impl Layout {
  pub fn new() -> Layout {
    let char_style = MonoTextStyleBuilder::new()
      .font(&profont::PROFONT_18_POINT)
      .text_color(WHITE)
      .background_color(BLACK)
      .build();
    let big_char_style = MonoTextStyleBuilder::new()
      .font(&BIGNUMBER_FONT)
      .text_color(WHITE)
      .background_color(BLACK)
      .build();
    let text_style = TextStyle::with_baseline(Baseline::Top);
    let bg_fill_style = PrimitiveStyle::with_fill(BLACK);
    let fg_fill_style = PrimitiveStyle::with_fill(WHITE);
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
    D: DrawTarget<Color = DPixelColor>
  {
    Rectangle::new(Point::new(0,0), Size::new(480, 320))
    .into_styled(self.bg_fill_style)
    .draw(display)?;
    Result::Ok(())
  }

  pub fn write_text<D>(&self, display: &mut D, loc: Point, content: &str) -> Result<Point, D::Error>
  where
    D: DrawTarget<Color = DPixelColor>
  {
    Text::with_text_style(content, loc, self.char_style, self.text_style)
      .draw(display)
  }

  pub fn write_big_text<D>(&self, display: &mut D, loc: Point, content: &str) -> Result<Point, D::Error>
  where
    D: DrawTarget<Color = DPixelColor>
  {
    Text::with_text_style(content, loc, self.big_char_style, self.text_style)
      .draw(display)
  }
  
  pub fn write_big_dp<D>(&self, display: &mut D, loc: Point) -> Result<Point, D::Error>
  where
    D: DrawTarget<Color = DPixelColor>
  {
    let diam = BIG_CHAR_HEIGHT / 7;
    let kern = 0;
    let topleft = loc + Point::new(kern, BIG_CHAR_HEIGHT-diam-4);
    Circle::new(topleft, diam as u32)
      .into_styled(self.fg_fill_style)
      .draw(display)?;
    Result::Ok(loc + Point::new(kern * 2 + diam, 0))
  }

  pub fn render_field<D, const N:usize>(&self, display: &mut D, cursor0: Point, field: &mut DisplayField<N>) -> Result<Point, D::Error>
  where
    D: DrawTarget<Color = DPixelColor>
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

pub struct Screens {
  layout: Layout,
  screens: AnyScreen,
}
impl Screens {
  pub fn new() -> Self {
    Screens {
      layout : Layout:: new(),
      screens: AnyScreen::Screen1(Screen1::new()),
    }
  }

  pub fn next_page<D>(&mut self, display: &mut D )-> Result<(), D::Error>
  where
  D: DrawTarget<Color = DPixelColor>
  {
    match &mut self.screens {
      AnyScreen::Screen1(_) => self.screens = AnyScreen::Screen2(Screen2::new()),
      AnyScreen::Screen2(_) => self.screens = AnyScreen::Screen1(Screen1::new()),
    }
    self.render_initial(display)
  }

  pub fn render_initial<D>(&mut self, display: &mut D )-> Result<(), D::Error>
  where
  D: DrawTarget<Color = DPixelColor>
  {
    match &mut self.screens {
      AnyScreen::Screen1(screen1) => screen1.render_initial(&self.layout, display),
      AnyScreen::Screen2(screen2) => screen2.render_initial(&self.layout, display),
    }
  }

  pub fn render_update<D>(&mut self, display: &mut D )-> Result<(), D::Error>
  where
  D: DrawTarget<Color = DPixelColor>
  {
    match &mut self.screens {
      AnyScreen::Screen1(screen1) => screen1.render_update(&self.layout, display),
      AnyScreen::Screen2(screen2) => screen2.render_update(&self.layout, display),
    }
  }

  pub fn update_gga(&mut self, ogga: Option<GGA>) {
    match &mut self.screens {
      AnyScreen::Screen1(screen1) => screen1.update_gga(ogga),
      AnyScreen::Screen2(screen2) => screen2.update_gga(ogga),
    }  
  }

  pub fn update_vtg(&mut self, vtg: VTG, stats: &SpeedStats) {
    match &mut self.screens {
      AnyScreen::Screen1(screen1) => screen1.update_vtg(vtg, stats),
      AnyScreen::Screen2(screen2) => screen2.update_vtg(vtg, stats),
    }  
  }

  pub fn update_vbat(&mut self, mv: u16 ) {
    match &mut self.screens {
      AnyScreen::Screen1(screen1) => (),
      AnyScreen::Screen2(screen2) => screen2.update_vbat(mv),
    }  
  }
}

enum AnyScreen {
  Screen1(Screen1),
  Screen2(Screen2),
}


pub struct Screen1 {
  speed_field : DisplayField<3>,  
  sats_field  : DisplayField<8>,
  units_field : DisplayField<2>,  
  no_signal_blink: bool,  
}

impl Screen1 {
  pub fn new() -> Self {
    Screen1 {
      speed_field: DisplayField::new(),
      sats_field: DisplayField::new(),
      units_field: DisplayField::new(),
      no_signal_blink: false,
    }
  }

  pub fn render_initial<D>(&mut self, layout: &Layout, display: &mut D )-> Result<(), D::Error>
  where
  D: DrawTarget<Color = DPixelColor>
  {
    write_field!(self.units_field, "kt").unwrap();
    write_field!(self.speed_field, "000").unwrap();
    layout.clear(display)?;
    Result::Ok(())
  }


  pub fn render_update<D>(&mut self, layout: &Layout, display: &mut D )-> Result<(), D::Error>
  where
  D: DrawTarget<Color = DPixelColor>
  {
    layout.render_field(display, Point::new(CHAR_WIDTH*2,CHAR_HEIGHT/2), &mut self.sats_field)?;
    layout.render_field(display, Point::new(CHAR_WIDTH*29,CHAR_HEIGHT/2), &mut self.units_field)?;

    self.render_speed(layout, display, layout.char_point(0, 2))?;

    Result::Ok(())
  }

  fn render_speed<D>(&mut self, layout: &Layout, display: &mut D,  loc: Point) -> Result<(), D::Error>
  where
    D: DrawTarget<Color = DPixelColor>
  {
    let mut cursor =  loc;
    let nextc = Point::new(BIG_CHAR_WIDTH, 0);

    if let Some(c) = self.speed_field.getdirtychar(0) {
      cursor = layout.write_big_text(display, cursor, c)?
    } else {
      cursor = cursor + nextc;
    }
    if let Some(c) = self.speed_field.getdirtychar(1) {
      cursor = layout.write_big_text(display, cursor, c)?
    } else {
      cursor = cursor + nextc;
    }
    cursor = layout.write_big_dp(display, cursor)?;
    if let Some(c) = self.speed_field.getdirtychar(2) {
      layout.write_big_text(display, cursor, c)?;
    }
    self.speed_field.clear_dirty();
    Result::Ok(())
  }


  pub fn update_gga(&mut self, ogga: Option<GGA>) {
    match ogga {
      Option::None => {

        if self.no_signal_blink {
          self.sats_field.clear();
        } else {
          write_field!(self.sats_field, "Sats: 0").unwrap();
        }
        self.no_signal_blink = ! self.no_signal_blink;
      },
      Option::Some(gga) => {
        write_field!(self.sats_field, "Sats: {:2}", gga.sat_in_use).unwrap();
      }
    }
  }

  pub fn update_vtg(&mut self, vtg: VTG, _stats: &SpeedStats) {
    write_field!(self.speed_field, "{:3}", (vtg.speed.as_knots() * 10.0).round() as u32).unwrap();
  }
}


pub struct Screen2 {
  sats_field  : DisplayField<18>,
  hdop_field  : DisplayField<18>,  
  lat_field   : DisplayField<18>,  
  lng_field   : DisplayField<18>,
  speed_field : DisplayField<18>,  
  max_speed_field : DisplayField<18>,
  vbat_field: DisplayField<6>,  

  no_signal_blink: bool,  
}

impl Screen2 {
  pub fn new() -> Self {
    Screen2 {
      sats_field: DisplayField::new(),
      hdop_field: DisplayField::new(),
      lat_field: DisplayField::new(),
      lng_field: DisplayField::new(),
      speed_field: DisplayField::new(),
      max_speed_field: DisplayField::new(),
      vbat_field: DisplayField::new(),
      no_signal_blink: false,
    }
  }

  pub fn render_initial<D>(&mut self, layout: &Layout, display: &mut D )-> Result<(), D::Error>
  where
  D: DrawTarget<Color = DPixelColor>
  {
    layout.clear(display)?;
    Result::Ok(())
  }


  pub fn render_update<D>(&mut self, layout: &Layout, display: &mut D )-> Result<(), D::Error>
  where
  D: DrawTarget<Color = DPixelColor>
  {
    let mut cursor = Point::new(0,0);
    let down = Point::new(0, CHAR_HEIGHT);

    layout.render_field(display, cursor, &mut self.sats_field)?;
    cursor = cursor + down;
    layout.render_field(display, cursor, &mut self.hdop_field)?;
    cursor = cursor + down;
    layout.render_field(display, cursor, &mut self.lat_field)?;
    cursor = cursor + down;
    layout.render_field(display, cursor, &mut self.lng_field)?;
    cursor = cursor + down;
    layout.render_field(display, cursor, &mut self.speed_field)?;
    cursor = cursor + down;
    layout.render_field(display, cursor, &mut self.max_speed_field)?;
    cursor = cursor + down;
    layout.render_field(display, cursor, &mut self.vbat_field)?;
    Result::Ok(())
  }

  pub fn update_gga(&mut self, ogga: Option<GGA>) {
    match ogga {
      Option::None => {

        if self.no_signal_blink {
          self.sats_field.clear();
        } else {
          write_field!(self.sats_field, "Sats: 0").unwrap();
        }
        self.no_signal_blink = ! self.no_signal_blink;
      },
      Option::Some(gga) => {
        write_field!(self.sats_field, "Sats: {:2}", gga.sat_in_use).unwrap();
        write_field!(self.hdop_field, "Hdop: {:5.1}", gga.hdop).unwrap();
        write_field!(self.lat_field, "Lat : {:12.6}", gga.latitude.as_f64()).unwrap();
        write_field!(self.lng_field, "Long: {:12.6}", gga.longitude.as_f64()).unwrap();
      }
    }
  }

  pub fn update_vtg(&mut self, vtg: VTG, stats: &SpeedStats) {
    write_field!(self.speed_field, "Spd : {:3.1}", vtg.speed.as_knots()).unwrap();
    write_field!(self.max_speed_field, "Max : {:3.1}", stats.max.as_knots()).unwrap();
  }

  pub fn update_vbat(&mut self, mv: u16 ) {
    write_field!(self.lng_field, "Vbat: {}mV", mv).unwrap();
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
