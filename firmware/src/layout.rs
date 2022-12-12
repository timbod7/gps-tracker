use embedded_graphics::primitives::Circle;
use crate::{u8writer::U8Writer, gps::GpsData};
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
use crate::write_field;

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
      screens: AnyScreen::Speed(SpeedScreen::new()),
    }
  }

  pub fn next_page<D>(&mut self, display: &mut D )-> Result<(), D::Error>
  where
  D: DrawTarget<Color = DPixelColor>
  {
    match &mut self.screens {
      AnyScreen::Speed(_) => self.screens = AnyScreen::Cog(CogScreen::new()),
      AnyScreen::Cog(_) => self.screens = AnyScreen::Misc(MiscScreen::new()),
      AnyScreen::Misc(_) => self.screens = AnyScreen::Speed(SpeedScreen::new()),

    }
    self.layout.clear(display)?;
    self.render(display)
  }

  pub fn render<D>(&mut self, display: &mut D )-> Result<(), D::Error>
  where
  D: DrawTarget<Color = DPixelColor>
  {
    match &mut self.screens {
      AnyScreen::Speed(screen1) => screen1.render(&self.layout, display),
      AnyScreen::Cog(screen2) => screen2.render(&self.layout, display),
      AnyScreen::Misc(screen3) => screen3.render(&self.layout, display),
    }
  }

  pub fn update_gps(&mut self, gps: &GpsData) {
    match &mut self.screens {
      AnyScreen::Speed(screen1) => screen1.update_gps(gps),
      AnyScreen::Cog(screen2) => screen2.update_gps(gps),
      AnyScreen::Misc(screen3) => screen3.update_gps(gps),
    }  
  }

  pub fn update_vbat(&mut self, mv: u16 ) {
    match &mut self.screens {
      AnyScreen::Speed(screen1) => screen1.update_vbat(mv),
      AnyScreen::Cog(screen2) => screen2.update_vbat(mv),
      AnyScreen::Misc(screen3) => screen3.update_vbat(mv),
    }  
  }
}

enum AnyScreen {
  Speed(SpeedScreen),
  Cog(CogScreen),
  Misc(MiscScreen),
}

pub struct StatusLine {
  sats_field  : DisplayField<8>,
  no_signal_blink: bool,  
  bat_percent : Option<u32>,
  label: DisplayField<4>,
}

impl StatusLine {
  pub fn new(label: &str) -> Self {
    StatusLine {
      sats_field: DisplayField::new(),
      no_signal_blink: false,
      bat_percent: None,
      label: DisplayField::from_str(label)
    }
  }
}

impl StatusLine {
  pub fn render<D>(&mut self, layout: &Layout, display: &mut D )-> Result<(), D::Error>
  where
  D: DrawTarget<Color = DPixelColor>
  {
    layout.render_field(display, Point::new(CHAR_WIDTH*2,CHAR_HEIGHT/2), &mut self.sats_field)?;
    if let Some(bat_percent) = self.bat_percent {
      render_battery_top_centre(display, layout, bat_percent)?;
    } 
    layout.render_field(display, Point::new(CHAR_WIDTH*29,CHAR_HEIGHT/2), &mut self.label)?;
    Result::Ok(())
  }

  pub fn update_vbat(&mut self, mv: u16 ) {
    self.bat_percent = Some(battery_percent(mv as u32));
  }

  pub fn update_gps(&mut self, gps: &GpsData) {
    if gps.sat_in_use == 0 {
      self.no_signal_blink = ! self.no_signal_blink;
      if self.no_signal_blink {
        self.sats_field.clear();
      } else {
        write_field!(self.sats_field, "Sats: 0").unwrap();
      }
    }
  }
}



pub struct SpeedScreen {
  status_line: StatusLine,
  speed_field : DisplayField<3>,  
}

impl SpeedScreen {
  pub fn new() -> Self {
    SpeedScreen {
      status_line: StatusLine::new("kt"),
      speed_field: DisplayField::new(),
    }
  }

  pub fn render<D>(&mut self, layout: &Layout, display: &mut D )-> Result<(), D::Error>
  where
  D: DrawTarget<Color = DPixelColor>
  {
    self.status_line.render(layout, display)?;
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


  pub fn update_gps(&mut self, gps: &GpsData) {
    self.status_line.update_gps(gps);
    write_field!(self.speed_field, "{:3}", (gps.speed * 10.0).round() as u32).unwrap();

  }

  pub fn update_vbat(&mut self, mv: u16 ) {
    self.status_line.update_vbat(mv);
  }
}

pub struct CogScreen {
  status_line: StatusLine,
  cog_field : DisplayField<3>,  
}

impl CogScreen {
  pub fn new() -> Self {
    CogScreen {
      status_line: StatusLine::new("COG"),
      cog_field: DisplayField::new(),
    }
  }

  pub fn render<D>(&mut self, layout: &Layout, display: &mut D )-> Result<(), D::Error>
  where
  D: DrawTarget<Color = DPixelColor>
  {
    self.status_line.render(layout, display)?;
    self.render_cog(layout, display, layout.char_point(0, 2))?;
    Result::Ok(())
  }

  fn render_cog<D>(&mut self, layout: &Layout, display: &mut D,  loc: Point) -> Result<(), D::Error>
  where
    D: DrawTarget<Color = DPixelColor>
  {
    let mut cursor =  loc;
    let nextc = Point::new(BIG_CHAR_WIDTH, 0);
    for i in 0..3 {
      if let Some(c) = self.cog_field.getdirtychar(i) {
        cursor = layout.write_big_text(display, cursor, c)?
      } else {
        cursor = cursor + nextc;
      }
    }
    self.cog_field.clear_dirty();
    Result::Ok(())
  }

  pub fn update_gps(&mut self, gps: &GpsData) {
    self.status_line.update_gps(gps);

    if let Some(course) = gps.course {
      let cog = course.round() as u16;
      write_field!(self.cog_field, "{:0>3}", cog).unwrap();
    } else {
      write_field!(self.cog_field, "---").unwrap();
    }
  }

  pub fn update_vbat(&mut self, mv: u16 ) {
    self.status_line.update_vbat(mv);
  }
}



pub struct MiscScreen {
  status_line: StatusLine,
  hdop_field  : DisplayField<18>,  
  lat_field   : DisplayField<18>,  
  lng_field   : DisplayField<18>,
  speed_field : DisplayField<18>,  
  max_speed_field : DisplayField<18>,
  vbat_field: DisplayField<18>,  
}

impl MiscScreen {
  pub fn new() -> Self {
    MiscScreen {
      status_line: StatusLine::new(""),
      hdop_field: DisplayField::from_str("Hdop:"),
      lat_field: DisplayField::from_str("Lat :"),
      lng_field: DisplayField::from_str("Lng :"),
      speed_field: DisplayField::from_str("Spd :"),
      max_speed_field: DisplayField::from_str("Max :"),
      vbat_field: DisplayField::from_str("Vbat:"),
    }
  }


  pub fn render<D>(&mut self, layout: &Layout, display: &mut D )-> Result<(), D::Error>
  where
  D: DrawTarget<Color = DPixelColor>
  {

    self.status_line.render(layout, display)?;

    let mut cursor = Point::new(CHAR_WIDTH * 2, CHAR_HEIGHT*3/2);
    let down = Point::new(0,  CHAR_HEIGHT);
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

  pub fn update_gps(&mut self, gps: &GpsData) {
    self.status_line.update_gps(gps);
    write_field!(self.speed_field, "Spd : {:3.1}", gps.speed).unwrap();
    write_field!(self.max_speed_field, "Max : {:3.1}", gps.max_speed).unwrap();
    match gps.hdop {
      Some(hdop) => write_field!(self.hdop_field, "Hdop: {:5.1}", hdop).unwrap(),
      None =>  write_field!(self.hdop_field, "Hdop: -    ").unwrap()
    }
    match gps.latitude {
      Some(latitude) => write_field!(self.lat_field, "Lat : {:12.6}", latitude).unwrap(),
      None =>  write_field!(self.lat_field, "Lat : -           ").unwrap(),
    }
    match gps.longitude {
      Some(longitude) => write_field!(self.lng_field, "Lat : {:12.6}", longitude).unwrap(),
      None =>  write_field!(self.lng_field, "Lat : -           ").unwrap(),
    }   
  }

  pub fn update_vbat(&mut self, mv: u16 ) {
    self.status_line.update_vbat(mv);
    write_field!(self.vbat_field, "Vbat: {}mV", mv).unwrap();
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
  pub fn from_str(s: &str)-> Self {
    let mut f = DisplayField::new();
    write_field!(f, "{}", s).unwrap();
    f
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

const BATTERY_WIDTH: u32 = 30;
const BATTERY_HEIGHT: u32 = 16;

const BATTERY_MINV : u32 = 3400;
const BATTERY_MAXV : u32 = 4200;

fn battery_percent(batmv: u32) -> u32 {
  if batmv < BATTERY_MINV {
    0
  } else if batmv > BATTERY_MAXV {
    100
  } else {
    (batmv - BATTERY_MINV) * 100 / (BATTERY_MAXV - BATTERY_MINV)
  }
}

pub fn render_battery_top_centre<D>(display: &mut D, layout: &Layout, percent: u32)-> Result<(), D::Error>
where
D: DrawTarget<Color = DPixelColor> {

  let loc = Point::new(
    ((display.bounding_box().size.width  - BATTERY_WIDTH) / 2) as i32,
    15,
  );
  render_battery(display, layout, loc, percent)
}

pub fn render_battery<D>(display: &mut D, layout: &Layout, loc: Point, percent: u32)-> Result<(), D::Error>
where
D: DrawTarget<Color = DPixelColor> {

  let size = Size::new(BATTERY_WIDTH,BATTERY_HEIGHT);
  let border = Size::new(2,2);
  let border2 = Size::new(3,3);
  let nib_size = Size::new(3,6);

  Rectangle::new(loc, size)
      .into_styled(layout.fg_fill_style)
      .draw(display)?;

  Rectangle::new(loc + Size::new(size.width, (size.height - nib_size.height)/2), nib_size)
      .into_styled(layout.fg_fill_style)
      .draw(display)?;

  Rectangle::new(loc + border, size - border * 2)
      .into_styled(layout.bg_fill_style)
      .draw(display)?;

  let mut fsize = size - border2 * 2;
  fsize.width = fsize.width * percent / 100;
  Rectangle::new(loc + border2, fsize)
      .into_styled(layout.fg_fill_style)
      .draw(display)?;
  Ok(())
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


