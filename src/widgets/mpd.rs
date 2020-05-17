use crate::widgets::widget::{Widget,Drawing,Located};

use std::io::{Write, BufRead, BufReader};
use std::net::TcpStream;

pub struct MPD {
    host: String,
    port: usize,
}

enum State {
    Playing(String),
    Stopped,
}

impl MPD {
    pub fn new(host: String, port: usize) -> MPD {
        MPD {host, port}
    }

    fn get_song(&self) -> Result<State, failure::Error> {
        let mut stream = TcpStream::connect(format!("{}:{}", self.host, self.port))?;

        let mut buf = String::new();
        BufReader::new(&stream).read_line(&mut buf)?;
        if !buf.starts_with("OK MPD") {
            return Err(format_err!("Unable to connect to MPD"));
        }
        buf.clear();

        stream.write(b"currentsong\n")?;
        let mut title = None;
        let mut artist = None;

        for l in BufReader::new(&stream).lines() {
            let line = l?;
            if line == "OK" {
                break;
            }

            if line.starts_with("Title") {
                if let Some(idx) = line.find(": ") {
                    title = line.get((idx + 2)..).map(|s| s.to_string());
                }
            }

            if line.starts_with("Artist") {
                if let Some(idx) = line.find(": ") {
                    artist = line.get((idx + 2)..).map(|s| s.to_string());
                }
            }
        }

        if let (Some(artist), Some(title)) = (artist, title) {
            Ok(State::Playing(format!("{}: {}", artist, title)))
        } else {
            Ok(State::Stopped)
        }
    }
}

impl Widget for MPD {
    fn draw(&self, d: &Drawing, loc: Located) -> i32 {
        match self.get_song() {
            Ok(State::Playing(song)) => loc.draw_text(d, &format!("[{}]", song)),
            Ok(State::Stopped) => loc.draw_text(d, &format!("[N/A]")),
            Err(_err) => loc.draw_text(d, &format!("[Error]")),
        }
    }
}
