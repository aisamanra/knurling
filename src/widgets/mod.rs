pub mod battery;
pub mod mpd;
pub mod standard;
pub mod widget;

pub use crate::widgets::widget::{Drawing, Located, Size, Widget};

const ALL_WIDGETS: [(
    &str,
    &dyn Fn(&toml::map::Map<String, toml::Value>) -> Result<Box<dyn Widget>, failure::Error>,
); 6] = [
    ("box", &|_| Ok(Box::new(standard::Time::new()))),
    ("battery", &|_| Ok(Box::new(battery::Battery::new()?))),
    ("caesura", &|_| Ok(Box::new(standard::Caesura))),
    ("mpd", &|config| {
        let host = config["host"]
            .as_str()
            .ok_or_else(|| format_err!("MPD host should be a string"))?;
        let port = config["port"]
            .as_integer()
            .ok_or_else(|| format_err!("MPD port should be an integer"))?;
        Ok(Box::new(mpd::MPD::new(host.to_string(), port as usize)))
    }),
    ("stdin", &|_| Ok(Box::new(standard::Stdin::new()))),
    ("time", &|_| Ok(Box::new(standard::Time::new()))),
];

pub fn mk_widget(
    name: &str,
    section: &toml::map::Map<String, toml::Value>,
) -> Result<Box<dyn Widget>, failure::Error> {
    for (n, f) in ALL_WIDGETS.iter() {
        if n == &name {
            return f(section);
        }
    }
    Err(format_err!("No widget type named {}", name))
}
