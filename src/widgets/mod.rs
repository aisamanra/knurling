pub mod battery;
pub mod standard;
pub mod widget;

pub use crate::widgets::widget::{Located,Drawing,Size,Widget};

const ALL_WIDGETS: [(&str, &dyn Fn(&toml::map::Map<String, toml::Value>) -> Result<Box<dyn Widget>, failure::Error>); 5] = [
    ("box", &|_| Ok(Box::new(standard::Time::new()))),
    ("battery", &|_| Ok(Box::new(battery::Battery::new()?))),
    ("caesura", &|_| Ok(Box::new(standard::Caesura))),
    ("stdin", &|_| Ok(Box::new(standard::Stdin::new()))),
    ("time", &|_| Ok(Box::new(standard::Time::new()))),
];

pub fn mk_widget(name: &str, section: &toml::map::Map<String, toml::Value>) -> Result<Box<dyn Widget>, failure::Error> {
    for (n, f) in ALL_WIDGETS.iter() {
        if n == &name {
            return f(section);
        }
    }
    Err(format_err!("No widget type named {}", name))
}
