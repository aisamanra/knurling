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
    pub fn draw_text(self, d: &Drawing, msg: &str) -> i32 {
        use pango::LayoutExt;
        d.lyt.set_text(msg);
        let (w, _) = d.lyt.get_size();
        d.ctx.move_to(self.target_x(d, w / pango::SCALE), d.buffer);
        pangocairo::functions::show_layout(d.ctx, d.lyt);
        w / pango::SCALE
    }

    pub fn target_x(self, d: &Drawing, w: i32) -> f64 {
        match self {
            Located::FromLeft(x) => x as f64,
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
    fn update_frequency(&self) -> Option<u64> {
        None
    }

    fn update(&mut self) {}

    fn draw(&self, d: &Drawing, loc: Located) -> i32;
}
