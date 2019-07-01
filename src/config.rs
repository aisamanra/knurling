use crate::widgets as w;

mod defaults {
    pub const BG_COLOR: (f64, f64, f64) = (0.1, 0.1, 0.1);
    pub const FG_COLOR: (f64, f64, f64) = (1.0, 1.0, 1.0);

    pub const FONT_FAMILY: &'static str = "Fira Mono";
    pub const FONT_SIZE: &'static str = "18";
}

pub struct Config {
    left: Vec<Box<w::Widget>>,
    right: Vec<Box<w::Widget>>,
    bg_color: (f64, f64, f64),
    fg_color: (f64, f64, f64),
    font: String,
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
        };
        let table = input.as_table().ok_or(format_err!("invalid config"))?;
        let widgets = &table["widgets"];
        let mut target = &mut conf.left;
        for section in widgets.as_array().ok_or(format_err!("invalid config"))? {
            let section = section.as_table().ok_or(format_err!("invalid config"))?;
            match section["name"].as_str().ok_or(format_err!(""))? {
                "box" => target.push(Box::new(w::SmallBox)),
                "battery" => target.push(Box::new(w::Battery::new()?)),
                "sep" => target = &mut conf.right,
                "stdin" => target.push(Box::new(w::Stdin::new())),
                "time" => target.push(Box::new(w::Time::new())),
                _ => (),
            }
        }
        if let Some(color) = table.get("background") {
            conf.bg_color = color_from_hex(color.as_str().ok_or(format_err!("`background` not a str"))?)?;
        }
        if let Some(color) = table.get("foreground") {
            conf.fg_color = color_from_hex(color.as_str().ok_or(format_err!("`foreground` not a str"))?)?;
        }
        if let Some(font) = table.get("font") {
            conf.font = font.as_str().ok_or(format_err!("`font` not a str"))?.to_string();
        }
        conf.right.reverse();
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

    pub fn draw(&self, ctx: &cairo::Context, layout: &pango::Layout, stdin: &str, size: w::Size) -> Result<(), failure::Error>{
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
            ctx: ctx,
            lyt: &layout,
            size,
            stdin,
        };

        let mut offset = 10;
        for w in self.left.iter() {
            offset += 10 + w.draw(&d, w::Located::FromLeft(offset));
        }
        offset = 10;
        for w in self.right.iter() {
            offset += 10 + w.draw(&d, w::Located::FromRight(offset));
        }

        Ok(())
    }

    pub fn font(&self) -> &str {
        &self.font
    }

    pub fn get_height(&self) -> i32 {
        use pango::LayoutExt;

        // we get the height here by making a fake surface, rendering
        // some text using our chosen font to it, and seeing how big it ends up being
        let surf = cairo::ImageSurface::create(
            cairo::Format::Rgb24, 0, 0).unwrap();
        let ctx = cairo::Context::new(&surf);
        let layout = pangocairo::functions::create_layout(&ctx).unwrap();
        layout.set_width(800 * pango::SCALE);
        let mut font = pango::FontDescription::from_string(self.font());
        font.set_weight(pango::Weight::Bold);
        layout.set_font_description(&font);
        layout.set_text("lj");
        let (_, h) = layout.get_size();
        (h / pango::SCALE) + 8
    }
}
