pub use crate::widgets::widget::{Located,Drawing,Size,Widget};

#[derive(Debug)]
pub struct Time {
    fmt: String,
}

impl Time {
    pub fn new() -> Time {
        Time {
            fmt: "%a %b %d %H:%M".to_string(),
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
