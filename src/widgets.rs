use pango::LayoutExt;

#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub wd: i32,
    pub ht: i32,
    pub xo: i32,
    pub yo: i32,
}

#[derive(Debug, Clone, Copy)]
pub enum Located {
    FromLeft(i32),
    FromRight(i32),
}

impl Located {
    fn draw_text(&self, d: &Drawing, msg: &str) -> i32 {
        d.lyt.set_text(msg);
        let (w, _) = d.lyt.get_size();
        d.ctx.move_to(self.target_x(d, w / pango::SCALE), d.buffer);
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
    pub stdin: &'t str,
    pub buffer: f64,
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
            fmt: format!("%a %b %d %H:%M"),
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
pub struct Stdin;

impl Stdin {
    pub fn new() -> Stdin {
        Stdin
    }
}

impl Widget for Stdin {
    fn draw(&self, d: &Drawing, loc: Located) -> i32 {
        loc.draw_text(d, &d.stdin)
    }
}

pub struct SmallBox;

impl Widget for SmallBox {
    fn draw(&self, d: &Drawing, loc: Located) -> i32 {
        let sz = d.size.ht - (d.buffer as i32 * 2);
        let x = loc.target_x(d, sz);
        d.ctx.rectangle(x, d.buffer, sz as f64, sz as f64);
        d.ctx.fill();
        sz
    }
}

pub struct Caesura;

impl Widget for Caesura {
    fn draw(&self, d: &Drawing, loc: Located) -> i32 {
        let x = loc.target_x(d, 1);
        d.ctx.move_to(x, d.buffer);
        d.ctx.line_to(x, d.size.ht as f64 - d.buffer);
        d.ctx.stroke();
        2
    }
}

pub struct Battery {
    file_list: Vec<std::path::PathBuf>,
    charging: Option<std::path::PathBuf>,
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
        let ac_path = std::path::Path::new("/sys/class/power_supply/AC/online");

        Ok(Battery {
            file_list: batteries,
            charging: if ac_path.exists() {
                Some(ac_path.to_path_buf())
            } else {
                None
            },
        })
    }

    fn is_charging(&self) -> Result<bool, failure::Error> {
        if let Some(path) = &self.charging {
            let is_connected: i32 = std::fs::read_to_string(path)?.trim().parse()?;
            Ok(is_connected != 0)
        } else {
            Ok(false)
        }
    }

    fn read_status(&self) -> Result<f64, failure::Error> {
        let charges: Result<Vec<i32>, failure::Error> = self
            .file_list
            .iter()
            .map(|path| Ok(std::fs::read_to_string(path)?.trim().parse()?))
            .collect();
        let charges = charges?;

        let len = charges.len() as f64;
        let sum: i32 = charges.into_iter().sum();
        Ok(sum as f64 / len / 100.0)
    }
}

impl Widget for Battery {
    fn draw(&self, d: &Drawing, loc: Located) -> i32 {
        let amt = self.read_status();
        let sz = d.size.ht - (d.buffer as i32 * 2);
        let x = loc.target_x(d, sz);
        match amt {
            _ if self.is_charging().unwrap_or(false) => d.ctx.set_source_rgb(0.5, 0.5, 1.0),
            Ok(x) if x < 0.1 => d.ctx.set_source_rgb(1.0, 0.0, 0.0),
            Ok(x) if x < 0.5 => d.ctx.set_source_rgb(1.0, 1.0, 0.0),
            Ok(_) => d.ctx.set_source_rgb(0.0, 1.0, 0.5),
            Err(_) => d.ctx.set_source_rgb(0.0, 0.0, 0.0),
        }

        d.ctx.rectangle(
            x,
            d.buffer * 2.0,
            sz as f64 * amt.unwrap_or(1.0),
            sz as f64 - d.buffer * 2.0,
        );
        d.ctx.fill();

        d.ctx.set_source_rgb(1.0, 1.0, 1.0);
        d.ctx
            .rectangle(x, d.buffer * 2.0, sz as f64, sz as f64 - (d.buffer * 2.0));
        d.ctx.stroke();

        sz
    }
}
