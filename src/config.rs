use crate::widgets as w;
use std::time;

mod defaults {
    pub const BG_COLOR: (f64, f64, f64) = (0.1, 0.1, 0.1);
    pub const FG_COLOR: (f64, f64, f64) = (1.0, 1.0, 1.0);

    pub const FONT_FAMILY: &str = "Fira Mono";
    pub const FONT_SIZE: &str = "18";
}

pub struct Config {
    left: Vec<WidgetWrapper>,
    right: Vec<WidgetWrapper>,
    bg_color: (f64, f64, f64),
    fg_color: (f64, f64, f64),
    font: String,
    height: i32,
    buffer: i32,
}

pub struct WidgetWrapper {
    update: Option<(time::Duration, time::SystemTime)>,
    widget: Box<dyn w::Widget>,
}

impl WidgetWrapper {
    fn new(mut widget: Box<dyn w::Widget>) -> WidgetWrapper {
        let update = if let Some(f) = widget.update_frequency() {
            widget.update();
            Some((time::Duration::new(f, 0), time::SystemTime::now()))
        } else {
            None
        };
        WidgetWrapper { update, widget }
    }

    fn update(&mut self) {
        if let Some((freq, ref mut last)) = self.update {
            if let Ok(since) = last.elapsed() {
                if since > freq {
                    self.widget.update();
                    *last = time::SystemTime::now();
                }
            }
        }
    }
}

pub fn color_from_hex(input: &str) -> Result<(f64, f64, f64), failure::Error> {
    let s = input.trim_start_matches("0x");
    let s = s.trim_start_matches(|c| !"ABCDEFabcdef0123456789".contains(c));
    match s.len() {
        6 => {
            let r = i64::from_str_radix(&s[0..2], 16)? as f64 / 255.0;
            let g = i64::from_str_radix(&s[2..4], 16)? as f64 / 255.0;
            let b = i64::from_str_radix(&s[4..6], 16)? as f64 / 255.0;
            Ok((r, g, b))
        }
        3 => {
            let r = i64::from_str_radix(&s[0..1], 16)? as f64 / 255.0;
            let g = i64::from_str_radix(&s[1..2], 16)? as f64 / 255.0;
            let b = i64::from_str_radix(&s[2..3], 16)? as f64 / 255.0;
            Ok((r, g, b))
        }
        _ => bail!("Unable to parse {} as a hex color literal", input),
    }
}

impl Config {
    pub fn from_toml(input: toml::Value) -> Result<Config, failure::Error> {
        let mut conf = Config {
            left: Vec::new(),
            right: Vec::new(),
            bg_color: defaults::BG_COLOR,
            fg_color: defaults::FG_COLOR,
            font: format!("{} {}", defaults::FONT_FAMILY, defaults::FONT_SIZE),
            height: 0,
            buffer: 0,
        };
        let table = input
            .as_table()
            .ok_or_else(|| format_err!("invalid config"))?;
        let widgets = &table["widgets"];
        let mut target = &mut conf.left;
        for section in widgets
            .as_array()
            .ok_or_else(|| format_err!("invalid config"))?
        {
            let section = section
                .as_table()
                .ok_or_else(|| format_err!("invalid config"))?;
            let name = section["name"]
                .as_str()
                .ok_or_else(|| format_err!("invalid config"))?;
            if name == "sep" {
                target = &mut conf.right;
            } else {
                target.push(WidgetWrapper::new(w::mk_widget(name, section)?));
            }
        }

        if let Some(color) = table.get("background") {
            conf.bg_color = color_from_hex(
                color
                    .as_str()
                    .ok_or_else(|| format_err!("`background` not a str"))?,
            )?;
        }
        if let Some(color) = table.get("foreground") {
            conf.fg_color = color_from_hex(
                color
                    .as_str()
                    .ok_or_else(|| format_err!("`foreground` not a str"))?,
            )?;
        }
        if let Some(font) = table.get("font") {
            conf.font = font
                .as_str()
                .ok_or_else(|| format_err!("`font` not a str"))?
                .to_string();
        }
        conf.right.reverse();

        let text_height = conf.calc_text_height();
        let buffer = text_height / 4;
        conf.height = conf.calc_text_height() + buffer * 2;
        conf.buffer = buffer;
        Ok(conf)
    }

    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Config, failure::Error> {
        let body = std::fs::read_to_string(path)?;
        let val = body.parse::<toml::Value>()?;
        Config::from_toml(val)
    }

    pub fn find_config() -> Result<Config, failure::Error> {
        if let Some(p) = xdg::BaseDirectories::new()?.find_config_file("knurling/knurling.toml") {
            return Config::from_file(p);
        }
        Err(format_err!("Unable to find `knurling.toml`"))
    }

    pub fn draw(
        &self,
        ctx: &cairo::Context,
        layout: &pango::Layout,
        stdin: &str,
        size: w::Size,
    ) -> Result<(), failure::Error> {
        // paint the background
        {
            let (r, g, b) = self.bg_color;
            ctx.set_source_rgb(r, g, b);
        }
        ctx.paint();

        // set the foreground color for drawing
        {
            let (r, g, b) = self.fg_color;
            ctx.set_source_rgb(r, g, b);
        }

        // set up a struct with everything that widgets need to draw
        let d = w::Drawing {
            ctx,
            lyt: &layout,
            size,
            stdin,
            buffer: self.buffer as f64,
        };

        let mut offset = 10;
        for w in self.left.iter() {
            offset += 10 + w.widget.draw(&d, w::Located::FromLeft(offset));
        }
        offset = 10;
        for w in self.right.iter() {
            offset += 10 + w.widget.draw(&d, w::Located::FromRight(offset));
        }

        Ok(())
    }

    pub fn update(&mut self) {
        for w in self.left.iter_mut() {
            w.update()
        }
        for w in self.right.iter_mut() {
            w.update()
        }
    }

    pub fn font(&self) -> &str {
        &self.font
    }

    pub fn get_height(&self) -> i32 {
        self.height
    }

    fn calc_text_height(&self) -> i32 {
        use pango::LayoutExt;

        // we get the height here by making a fake surface, rendering
        // some text using our chosen font to it, and seeing how big it ends up being
        let surf = cairo::ImageSurface::create(cairo::Format::Rgb24, 0, 0).unwrap();
        let ctx = cairo::Context::new(&surf);
        let layout = pangocairo::functions::create_layout(&ctx).unwrap();
        layout.set_width(800 * pango::SCALE);
        let mut font = pango::FontDescription::from_string(self.font());
        font.set_weight(pango::Weight::Bold);
        layout.set_font_description(&font);
        layout.set_text("lj");
        let (_, h) = layout.get_size();
        h / pango::SCALE
    }
}
