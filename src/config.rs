use crate::widgets as w;

pub struct Config {
    left: Vec<Box<w::Widget>>,
    right: Vec<Box<w::Widget>>,
}

impl Config {
    pub fn from_toml(input: toml::Value) -> Result<Config, failure::Error> {
        let mut conf = Config { left: Vec::new(), right: Vec::new() };
        let widgets = &input.as_table().ok_or(format_err!("invalid config"))?["widgets"];
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
        // the background is... gray-ish? this'll be configurable eventually
        ctx.set_source_rgb(0.1, 0.1, 0.1);
        ctx.paint();

        // and the text is white
        ctx.set_source_rgb(1.0, 1.0, 1.0);

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
}
