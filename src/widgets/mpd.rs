use crate::widgets::widget::{Drawing, Located, Widget};

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

pub struct MPD {
    host: String,
    port: usize,
    last_state: State,
}

enum State {
    Playing(String),
    Stopped,
}

impl MPD {
    pub fn new(host: String, port: usize) -> MPD {
        let last_state = State::Stopped;
        MPD {
            host,
            port,
            last_state,
        }
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
        match self.last_state {
            State::Playing(ref song) => loc.draw_text(d, &format!("[{}]", song)),
            State::Stopped => loc.draw_text(d, &format!("[N/A]")),
        }
    }

    fn update_frequency(&self) -> Option<u64> {
        Some(5)
    }

    fn update(&mut self) {
        match self.get_song() {
            Ok(state) => self.last_state = state,
            Err(err) => eprintln!("Failed to update MPD status: {}", err),
        }
    }
}
