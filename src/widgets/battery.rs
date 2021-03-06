use crate::widgets::widget::{Drawing, Located, Widget};

pub struct Battery {
    file_list: Vec<std::path::PathBuf>,
    charging: Option<std::path::PathBuf>,
    last_status: f64,
    last_charging: bool,
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
            last_status: 1.0f64,
            last_charging: false,
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
        let amt = self.last_status;
        let sz = d.size.ht - (d.buffer as i32 * 2);
        let x = loc.target_x(d, sz);
        match amt {
            _ if self.last_charging => d.ctx.set_source_rgb(0.5, 0.5, 1.0),
            x if x < 0.1 => d.ctx.set_source_rgb(1.0, 0.0, 0.0),
            x if x < 0.5 => d.ctx.set_source_rgb(1.0, 1.0, 0.0),
            _ => d.ctx.set_source_rgb(0.0, 1.0, 0.5),
        }

        d.ctx.rectangle(
            x,
            d.buffer * 2.0,
            sz as f64 * amt,
            sz as f64 - d.buffer * 2.0,
        );
        d.ctx.fill();

        d.ctx.set_source_rgb(1.0, 1.0, 1.0);
        d.ctx
            .rectangle(x, d.buffer * 2.0, sz as f64, sz as f64 - (d.buffer * 2.0));
        d.ctx.stroke();

        sz
    }

    fn update_frequency(&self) -> Option<u64> {
        Some(10)
    }

    fn update(&mut self) {
        if let Ok(status) = self.read_status() {
            self.last_status = status;
        }

        if let Ok(charging) = self.is_charging() {
            self.last_charging = charging;
        }
    }
}
