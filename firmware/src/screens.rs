use crate::gps::{GpsData, GpsTime};
use crate::layout::{DPixelColor, DisplayField, Layout, BIG_CHAR_WIDTH, CHAR_HEIGHT, CHAR_WIDTH};
use crate::u8writer::U8Writer;
use crate::write_field;

use core::fmt::Write;
use embedded_graphics::{
    prelude::*,
    primitives::{Line, Rectangle},
};
use micromath::F32Ext;

pub enum Update<'a> {
    Gps(&'a GpsData),
    Vbat(u16),
}

pub struct Screens {
    layout: Layout,
    screens: AnyScreen,
}
impl Screens {
    pub fn new() -> Self {
        Screens {
            layout: Layout::new(),
            screens: AnyScreen::Speed(SpeedScreen::new()),
            //screens: AnyScreen::SpeedDetails(SpeedDetailsScreen::new()),
        }
    }

    pub fn next_page<D>(&mut self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        match &mut self.screens {
            AnyScreen::Speed(_) => self.screens = AnyScreen::Stats(StatsScreen::new()),
            AnyScreen::Stats(_) => self.screens = AnyScreen::Cog(CogScreen::new()),
            AnyScreen::Cog(_) => self.screens = AnyScreen::Misc(MiscScreen::new()),
            AnyScreen::Misc(_) => self.screens = AnyScreen::Speed(SpeedScreen::new()),
        }
        self.layout.clear(display)?;
        self.render(display)
    }

    pub fn render<D>(&mut self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        match &mut self.screens {
            AnyScreen::Speed(s) => s.render(&self.layout, display),
            AnyScreen::Stats(s) => s.render(&self.layout, display),
            AnyScreen::Cog(s) => s.render(&self.layout, display),
            AnyScreen::Misc(s) => s.render(&self.layout, display),
        }
    }

    pub fn update_gps(&mut self, gps: &GpsData) {
        self.update(&Update::Gps(gps));
    }

    pub fn update_vbat(&mut self, mv: u16) {
        self.update(&Update::Vbat(mv));
    }

    pub fn update(&mut self, update: &Update) {
        match &mut self.screens {
            AnyScreen::Speed(s) => s.update(update),
            AnyScreen::Stats(s) => s.update(update),
            AnyScreen::Cog(s) => s.update(update),
            AnyScreen::Misc(s) => s.update(update),
        }
    }
}

enum AnyScreen {
    Speed(SpeedScreen),
    Stats(StatsScreen),
    Cog(CogScreen),
    Misc(MiscScreen),
}

pub struct StatusLine {
    sats_field: DisplayField<8>,
    sats_blink: bool,
    bat_percent: Option<u32>,
    label: DisplayField<4>,
}

impl StatusLine {
    pub fn new(label: &str) -> Self {
        StatusLine {
            sats_field: DisplayField::new(),
            sats_blink: false,
            bat_percent: None,
            label: DisplayField::from_str(label),
        }
    }
}

impl StatusLine {
    pub fn render<D>(&mut self, layout: &Layout, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        layout.render_field(
            display,
            Point::new(CHAR_WIDTH * 2, CHAR_HEIGHT / 2),
            &mut self.sats_field,
        )?;
        if let Some(bat_percent) = self.bat_percent {
            render_battery_top_centre(display, layout, bat_percent)?;
        }
        layout.render_field(
            display,
            Point::new(CHAR_WIDTH * 29, CHAR_HEIGHT / 2),
            &mut self.label,
        )?;
        Result::Ok(())
    }

    pub fn update(&mut self, update: &Update) {
        match update {
            Update::Gps(gps) => self.update_gps(gps),
            Update::Vbat(vbat) => self.update_vbat(*vbat),
        }
    }

    pub fn update_vbat(&mut self, mv: u16) {
        self.bat_percent = Some(battery_percent(mv as u32));
    }

    pub fn update_gps(&mut self, gps: &GpsData) {
        self.sats_blink = !self.sats_blink;
        if gps.sat_in_use == 0 && self.sats_blink {
            self.sats_field.clear();
        } else {
            write_field!(self.sats_field, "Sats: {}", gps.sat_in_use).unwrap();
        }
    }
}

pub struct SpeedScreen {
    status_line: StatusLine,
    speed_digits: [Updateable<u8>; 3],
}

impl SpeedScreen {
    pub fn new() -> Self {
        SpeedScreen {
            status_line: StatusLine::new("kt"),
            speed_digits: [Updateable::new(0), Updateable::new(0), Updateable::new(0)],
        }
    }

    pub fn render<D>(&mut self, layout: &Layout, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        self.status_line.render(layout, display)?;
        self.render_speed(layout, display, layout.char_point(0, 2))?;
        Result::Ok(())
    }

    fn render_speed<D>(
        &mut self,
        layout: &Layout,
        display: &mut D,
        loc: Point,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        let mut cursor = loc;
        let nextc = Point::new(BIG_CHAR_WIDTH, 0);

        if let Some(d) = self.speed_digits[0].updated() {
            let mut c = char::from_digit(*d as u32, 10).unwrap();
            c = if c == '0' { ' ' } else { c };
            cursor = layout.write_big_char(display, cursor, c)?;
        } else {
            cursor = cursor + nextc;
        }
        if let Some(d) = self.speed_digits[1].updated() {
            let c = char::from_digit(*d as u32, 10).unwrap();
            cursor = layout.write_big_char(display, cursor, c)?;
        } else {
            cursor = cursor + nextc;
        }
        cursor = layout.write_big_dp(display, cursor)?;
        if let Some(d) = self.speed_digits[2].updated() {
            let c = char::from_digit(*d as u32, 10).unwrap();
            layout.write_big_char(display, cursor, c)?;
        }
        Result::Ok(())
    }

    pub fn update(&mut self, update: &Update) {
        self.status_line.update(update);
        match update {
            Update::Gps(gps) => self.update_gps(gps),
            _ => (),
        }
    }

    fn update_gps(&mut self, gps: &GpsData) {
        let speed = (gps.speed * 10.0).round() as u32;
        self.speed_digits[0].set(((speed / 100) % 10) as u8);
        self.speed_digits[1].set(((speed / 10) % 10) as u8);
        self.speed_digits[2].set((speed % 10) as u8);
    }
}

pub struct StatsScreen {
    max_speed: Updateable<f32>,
    max_avg_speed: Updateable<f32>,
    distance_nm: Updateable<f32>,
    time: Updateable<Option<GpsTime>>,
}

impl StatsScreen {
    pub fn new() -> Self {
        StatsScreen {
            max_speed: Updateable::new(0.0),
            max_avg_speed: Updateable::new(0.0),
            distance_nm: Updateable::new(0.0),
            time: Updateable::new(None),
        }
    }

    pub fn render<D>(&mut self, layout: &Layout, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        let (x0, x1, y0, y1) = (0, 205, 35, 155);
        let tl = Point::new(x0, y0);
        let tr = Point::new(x1, y0);
        let bl = Point::new(x0, y1);
        let br = Point::new(x1, y1);
        let labeld = Point::new(10, -30);

        Line::new(Point::new(0, 120), Point::new(400, 120))
            .into_styled(layout.fg_fill_style)
            .draw(display)?;
        Line::new(Point::new(200, 0), Point::new(200, 240))
            .into_styled(layout.fg_fill_style)
            .draw(display)?;

        layout.write_text(display, tl + labeld, "max kt")?;
        Self::render_f32_dd_d(layout, display, tl, &mut self.max_speed)?;

        layout.write_text(display, tr + labeld, "max kt avg10")?;
        Self::render_f32_dd_d(layout, display, tr, &mut self.max_avg_speed)?;

        layout.write_text(display, bl + labeld, "dist nm")?;
        Self::render_f32_dd_d(layout, display, bl, &mut self.distance_nm)?;

        layout.write_text(display, br + labeld, "time")?;
        Self::render_time(layout, display, br, &mut self.time)?;

        Result::Ok(())
    }

    fn render_time<D>(
        layout: &Layout,
        display: &mut D,
        loc: Point,
        value: &mut Updateable<Option<GpsTime>>,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        let mut buf: [u8; 20] = [0; 20];
        let mut w = U8Writer::new(&mut buf);
        if let Some(otime) = value.updated() {
            if let Some(time) = otime {
                write!(w, "{:02}:{:02}:{:02}", time.hour, time.min, time.sec).unwrap();
            } else {
                write!(w, "--:--:--").unwrap();
            }
            let dloc = Point::new(40, 20);
            layout.write_text(display, loc + dloc, w.as_str())?;
        }
        Ok(())
    }

    fn render_f32_dd_d<D>(
        layout: &Layout,
        display: &mut D,
        loc: Point,
        value: &mut Updateable<f32>,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        if let Some(value) = value.updated() {
            let mut buf: [u8; 8] = [0; 8];
            let mut w = U8Writer::new(&mut buf);

            let value = (value * 10.0).round() as u32;
            w.write_number(2, ' ', value / 10).unwrap();
            w.write_char('.').unwrap();
            w.write_number(1, '0', value % 10).unwrap();
            layout.write_med_text(display, loc, w.as_str())?;
        }
        Result::Ok(())
    }

    pub fn update(&mut self, update: &Update) {
        match update {
            Update::Gps(gps) => {
                self.max_speed.set(gps.max_speed);
                self.max_avg_speed.set(gps.max_avg_speed);
                self.distance_nm.set(gps.distance_m as f32 / 1852.0);
                self.time.set(gps.time.clone());
            }
            _ => (),
        }
    }
}

pub struct CogScreen {
    status_line: StatusLine,
    cog_digits: [Updateable<Option<u8>>; 3],
}

impl CogScreen {
    pub fn new() -> Self {
        CogScreen {
            status_line: StatusLine::new("COG"),
            cog_digits: [
                Updateable::new(None),
                Updateable::new(None),
                Updateable::new(None),
            ],
        }
    }

    pub fn render<D>(&mut self, layout: &Layout, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        self.status_line.render(layout, display)?;
        self.render_cog(layout, display, layout.char_point(0, 2))?;
        Result::Ok(())
    }

    fn render_cog<D>(
        &mut self,
        layout: &Layout,
        display: &mut D,
        loc: Point,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        let mut cursor = loc;
        let nextc = Point::new(BIG_CHAR_WIDTH, 0);
        for i in 0..3 {
            if let Some(od) = self.cog_digits[i].updated() {
                let c = match *od {
                    Some(d) => char::from_digit(d as u32, 10).unwrap(),
                    None => '-',
                };
                cursor = layout.write_big_char(display, cursor, c)?
            } else {
                cursor = cursor + nextc;
            }
        }
        Result::Ok(())
    }

    pub fn update(&mut self, update: &Update) {
        self.status_line.update(update);
        match update {
            Update::Gps(gps) => self.update_gps(gps),
            _ => (),
        }
    }

    fn update_gps(&mut self, gps: &GpsData) {
        self.status_line.update_gps(gps);

        if let Some(course) = gps.course {
            let cog = course.round() as u32;
            self.cog_digits[0].set(Some(((cog / 100) % 10) as u8));
            self.cog_digits[1].set(Some(((cog / 10) % 10) as u8));
            self.cog_digits[2].set(Some((cog % 10) as u8));
        } else {
            self.cog_digits[0].set(None);
            self.cog_digits[1].set(None);
            self.cog_digits[2].set(None);
        }
    }
}

pub struct MiscScreen {
    status_line: StatusLine,
    hdop_field: DisplayField<18>,
    lat_field: DisplayField<18>,
    lng_field: DisplayField<18>,
    speed_field: DisplayField<18>,
    max_speed_field: DisplayField<18>,
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

    pub fn render<D>(&mut self, layout: &Layout, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = DPixelColor>,
    {
        self.status_line.render(layout, display)?;

        let mut cursor = Point::new(CHAR_WIDTH * 2, CHAR_HEIGHT * 3 / 2);
        let down = Point::new(0, CHAR_HEIGHT);
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

    pub fn update(&mut self, update: &Update) {
        self.status_line.update(update);
        match update {
            Update::Gps(gps) => self.update_gps(gps),
            Update::Vbat(mv) => self.update_vbat(*mv),
        }
    }

    fn update_gps(&mut self, gps: &GpsData) {
        self.status_line.update_gps(gps);
        write_field!(self.speed_field, "Spd : {:3.1}", gps.speed).unwrap();
        write_field!(self.max_speed_field, "Max : {:3.1}", gps.max_speed).unwrap();
        match gps.hdop {
            Some(hdop) => write_field!(self.hdop_field, "Hdop: {:5.1}", hdop).unwrap(),
            None => write_field!(self.hdop_field, "Hdop: -    ").unwrap(),
        }
        match gps.latitude {
            Some(latitude) => write_field!(self.lat_field, "Lat : {:12.6}", latitude).unwrap(),
            None => write_field!(self.lat_field, "Lat : -           ").unwrap(),
        }
        match gps.longitude {
            Some(longitude) => write_field!(self.lng_field, "Lat : {:12.6}", longitude).unwrap(),
            None => write_field!(self.lng_field, "Lat : -           ").unwrap(),
        }
    }

    fn update_vbat(&mut self, mv: u16) {
        self.status_line.update_vbat(mv);
        write_field!(self.vbat_field, "Vbat: {}mV", mv).unwrap();
    }
}

const BATTERY_WIDTH: u32 = 30;
const BATTERY_HEIGHT: u32 = 16;

const BATTERY_MINV: u32 = 3400;
const BATTERY_MAXV: u32 = 4200;

fn battery_percent(batmv: u32) -> u32 {
    if batmv < BATTERY_MINV {
        0
    } else if batmv > BATTERY_MAXV {
        100
    } else {
        (batmv - BATTERY_MINV) * 100 / (BATTERY_MAXV - BATTERY_MINV)
    }
}

pub fn render_battery_top_centre<D>(
    display: &mut D,
    layout: &Layout,
    percent: u32,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = DPixelColor>,
{
    let loc = Point::new(
        ((display.bounding_box().size.width - BATTERY_WIDTH) / 2) as i32,
        15,
    );
    render_battery(display, layout, loc, percent)
}

pub fn render_battery<D>(
    display: &mut D,
    layout: &Layout,
    loc: Point,
    percent: u32,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = DPixelColor>,
{
    let size = Size::new(BATTERY_WIDTH, BATTERY_HEIGHT);
    let border = Size::new(2, 2);
    let border2 = Size::new(3, 3);
    let nib_size = Size::new(3, 6);

    Rectangle::new(loc, size)
        .into_styled(layout.fg_fill_style)
        .draw(display)?;

    Rectangle::new(
        loc + Size::new(size.width, (size.height - nib_size.height) / 2),
        nib_size,
    )
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

pub struct Updateable<T> {
    value: T,
    updated: bool,
}

impl<T: PartialEq> Updateable<T> {
    pub fn new(value: T) -> Self {
        Updateable {
            value,
            updated: true,
        }
    }

    pub fn set(&mut self, value: T) {
        if self.value != value {
            self.value = value;
            self.updated = true;
        }
    }

    pub fn updated(&mut self) -> Option<&T> {
        if self.updated {
            self.updated = false;
            Some(&self.value)
        } else {
            None
        }
    }
}
