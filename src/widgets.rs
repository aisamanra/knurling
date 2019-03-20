use crate::window::Size;
use pango::LayoutExt;

#[derive(Debug,Clone,Copy)]
pub enum Located {
    FromLeft(i32),
    FromRight(i32),
}

pub struct Config<'r> {
    pub left: Vec<&'r Widget>,
    pub right: Vec<&'r Widget>,
}

impl<'r> Config<'r> {
    pub fn draw(&self, d: &Drawing) {
        let mut offset = 10;
        for w in self.left.iter() {
            offset += 10 + w.draw(d, Located::FromLeft(offset));
        }
        offset = 10;
        for w in self.right.iter() {
            offset += 10 + w.draw(d, Located::FromRight(offset));
        }
    }
}

impl Located {
    fn draw_text(&self, d: &Drawing, msg: &str) -> i32 {
        d.lyt.set_text(msg);
        let (w, _) = d.lyt.get_size();
        d.ctx.move_to(self.target_x(d, w / pango::SCALE), 4.0);
        pangocairo::functions::show_layout(d.ctx, d.lyt);
        w / pango::SCALE
    }

    fn target_x(&self, d: &Drawing, w: i32) -> f64 {
        match self {
            Located::FromLeft(x) => *x as f64,
            Located::FromRight(x) => (d.size.wd - (x + w)) as f64,
        }
    }
}

pub struct Drawing<'t> {
    pub ctx: &'t cairo::Context,
    pub lyt: &'t pango::Layout,
    pub size: Size,
}

pub trait Widget {
    fn draw(&self, d: &Drawing, loc: Located) -> i32;
}

#[derive(Debug)]
pub struct Time {
    fmt: String,
}

impl Time {
    pub fn new() -> Time {
        Time {
            fmt: format!("%a %b %d %H:%M")
        }
    }
}

impl Widget for Time {
    fn draw(&self, d: &Drawing, loc: Located) -> i32 {
        let now = chrono::Local::now();
        loc.draw_text(d, &format!("{}", &now.format(&self.fmt)))
    }
}


#[derive(Debug)]
pub struct Text<'t> {
    text: &'t str,
}

impl<'t> Text<'t> {
    pub fn new(text: &str) -> Text {
        Text { text }
    }
}

impl<'t> Widget for Text<'t> {
    fn draw(&self, d: &Drawing, loc: Located) -> i32 {
        loc.draw_text(d, &self.text)
    }
}


pub struct SmallBox;

impl Widget for SmallBox {
    fn draw(&self, d: &Drawing, loc: Located) -> i32 {
        let sz = d.size.ht - 8;
        let x = loc.target_x(d, sz);
        d.ctx.rectangle(x, 4.0, sz as f64, sz as f64);
        d.ctx.fill();
        sz
    }
}



pub struct Battery {
    file_list: Vec<std::path::PathBuf>,
}

impl Battery {
    pub fn new() -> Result<Battery, failure::Error> {
        use std::fs;

        let mut batteries = Vec::new();
        for entry in fs::read_dir("/sys/class/power_supply")? {
            let e = entry?;
            if e.file_name().to_string_lossy().starts_with("BAT") {
                let mut path = e.path();
                path.push("capacity");
                batteries.push(path);
            }
        }

        Ok(Battery {
            file_list: batteries,
        })
    }

    fn read_status(&self) -> Result<f64, failure::Error> {
        let charges: Result<Vec<i32>, failure::Error> =
            self.file_list.iter().map(|path| {
            Ok(std::fs::read_to_string(path)?.trim().parse()?)
            }).collect();
        let charges = charges?;

        let len = charges.len() as f64;
        let sum: i32 = charges.into_iter().sum();
        Ok(sum as f64 / len / 100.0)
    }
}

impl Widget for Battery {
    fn draw(&self, d: &Drawing, loc: Located) -> i32 {
        let amt = self.read_status().unwrap();
        let sz = d.size.ht - 8;
        let x = loc.target_x(d, sz);
        if amt < 0.1 {
            d.ctx.set_source_rgb(1.0, 0.0, 0.0);
        } else if amt < 0.5 {
            d.ctx.set_source_rgb(1.0, 1.0, 0.0);
        } else {
            d.ctx.set_source_rgb(0.0, 1.0, 0.5);
        }
        d.ctx.rectangle(x, 8.0, sz as f64 * amt, sz as f64 - 8.0);
        d.ctx.fill();

        d.ctx.set_source_rgb(1.0, 1.0, 1.0);
        d.ctx.rectangle(x, 8.0, sz as f64, sz as f64 - 8.0);
        d.ctx.stroke();

        sz
    }
}
