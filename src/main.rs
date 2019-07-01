#[macro_use]
extern crate failure;

mod config;
mod widgets;
mod window;

use std::os::unix::io::AsRawFd;
use pango::LayoutExt;

use widgets::Size;
use window::{Display,Event,Window};

fn main() -> Result<(), failure::Error> {
    // set up the display and the window
    let config = config::Config::from_file("sample.toml")?;
    let mut d = Display::create()?;
    let mut ws = Vec::new();
    for (x_off, wd) in d.get_widths()? {
        let size = Size { wd, ht: 36, xo: x_off, yo: 0 };
        let mut w = Window::create(&d, size)?;
        // set some window-manager properties: this is a dock
        w.change_property("_NET_WM_WINDOW_TYPE", &["_NET_WM_WINDOW_TYPE_DOCK"])?;
        // ...and should push other windows out of the way
        w.change_property("_NET_WM_STRUT", &[x_off as i64, 0, size.ht as i64, 0])?;
        w.change_property(
            "_NET_WM_STRUT_PARTIAL",
            &[ 0, 0, size.ht as i64, 0,
               0, 0, 0, 0,
               0, size.wd as i64, 0, 0,
            ],
        )?;

        // we won't ever see this, but for good measure.
        w.set_title("rbar")?;
        // we care about some input events!
        w.set_input_masks()?;
        w.set_protocols()?;
        // and now show it!
        w.map();
        ws.push(w);
    }

    // we do some grossness with file descriptors later, so we need
    // the file descriptors we care about here
    let window_fds: Vec<i32> = ws.iter_mut().map({ |w| w.get_fd() }).collect();
    let stdin_fd = std::io::stdin().as_raw_fd();
    let mut fds = unsafe { std::mem::uninitialized() };

    // To begin with, our left-hand side---which normally is whatever
    // was last passed in on stdin---will start as a generic
    // message...
    let mut input = format!("Loading...");
    // And let's get a buffered stdin handle now
    let mut stdin = std::io::BufReader::new(std::io::stdin());

    // In the absence of other events, let's refresh every five
    // seconds. Or whatever.
    let mut timer = libc::timeval {
        tv_sec: 5,
        tv_usec: 0,
    };

    let mut ctxs = Vec::new();
    for w in ws.iter_mut() {
        // let's grab the cairo context here
        let surf = w.get_cairo_surface();
        let ctx = cairo::Context::new(&surf);


        let layout = pangocairo::functions::create_layout(&ctx)
            .ok_or(format_err!("unable to create layout"))?;

        // allow for the whole width of the bar, minus a small fixed amount
        layout.set_width((w.width - 20) * pango::SCALE);
        // this should also be configurable, but Fira Mono is a good font
        let mut font = pango::FontDescription::from_string("Fira Mono 18");
        font.set_weight(pango::Weight::Bold);
        layout.set_font_description(&font);

        // do an initial pass at drawing the bar!
        config.draw(&ctx, &layout, &input, w.size())?;

        ctxs.push((ctx, layout, w.size()));
    }


    let max_fd = window_fds.iter().max().unwrap_or(&0) + 1;
    // we're gonna keep looping until we don't
    loop {
        unsafe {
            // set up the FD set to be the X11 fd and the state of stdin
            libc::FD_ZERO(&mut fds);
            for fd in window_fds.iter() {
                libc::FD_SET(*fd, &mut fds);
            }
            libc::FD_SET(stdin_fd, &mut fds);
            timer.tv_sec = 5;

            // this will block until there's input on either of the
            // above FDs or until five seconds have passed, whichever comes first
            libc::select(
                max_fd,
                &mut fds,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut timer,
            );
        }

        // if we _did_ have input on stdin, then read it in: that'll
        // be our new left-hand text
        if unsafe { libc::FD_ISSET(stdin_fd, &mut fds) } {
            use std::io::BufRead;
            input = String::new();
            stdin.read_line(&mut input)?;
            if input.len() == 0 {
                break;
            }
            for (ctx, layout, sz) in ctxs.iter() {
                config.draw(&ctx, &layout, &input, *sz)?;
            }
        }

        // if we have X11 events, handle them. If any one was a quit
        // event, then just... quit.
        for w in ws.iter_mut() {
            while w.has_events() {
                match w.handle() {
                    Some(Event::QuitEvent) => break,
                    _e => (),
                }
            }
        }

        for (ctx, layout, sz) in ctxs.iter() {
            // otherwise, draw the thing!
            config.draw(&ctx, &layout, &input, *sz)?;
        }
    }

    Ok(())
}
