mod window;

use std::os::unix::io::AsRawFd;
use pango::LayoutExt;

use window::{Display,Event,Size,Window};

fn main() {
    // set up the display and the window
    let mut d = Display::create();
    let size = Size {
        wd: d.get_width(),
        // TODO: this should be a function of font size
        ht: 36,
    };
    let mut w = Window::create(d, size);
    // set some window-manager properties: this is a dock
    w.change_property("_NET_WM_WINDOW_TYPE", &["_NET_WM_WINDOW_TYPE_DOCK"]);
    // ...and should push other windows out of the way
    w.change_property("_NET_WM_STRUT", &[0i64, 0, size.ht as i64, 0],);
    w.change_property(
        "_NET_WM_STRUT_PARTIAL",
        &[ 0, 0, size.ht as i64, 0,
           0, 0, 0, 0,
           0, size.wd as i64, 0, 0,
        ],
    );

    // we won't ever see this, but for good measure.
    w.set_title("rbar");
    // we care about some input events!
    w.set_input_masks();
    w.set_protocols();
    // and now show it!
    w.map();

    // let's grab the cairo context here
    let surf = w.get_cairo_surface();
    let ctx = cairo::Context::new(&surf);

    // we do some grossness with file descriptors later, so we need
    // the file descriptors we care about here
    let window_fd = w.get_fd();
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
    // do an initial pass at drawing the bar!
    draw(&ctx, &input, size);

    // we're gonna keep looping until we don't
    loop {
        unsafe {
            // set up the FD set to be the X11 fd and the state of stdin
            libc::FD_ZERO(&mut fds);
            libc::FD_SET(window_fd, &mut fds);
            libc::FD_SET(stdin_fd, &mut fds);

            // this will block until there's input on either of the
            // above FDs or until five seconds have passed, whichever comes first
            libc::select(
                window_fd + 1,
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
            stdin.read_line(&mut input).unwrap();
            if input == "" {
                break;
            }
            draw(&ctx, &input, size);
        }

        // if we have X11 events, handle them. If any one was a quit
        // event, then just... quit.
        while w.has_events() {
            match w.handle() {
                Some(Event::QuitEvent) => break,
                _e => (),
            }
        }

        // otherwise, draw the thing!
        draw(&ctx, &input, size);
    }
}


/// Do our Cairo drawing. This needs to be refactored to allow for
/// more configurability in terms of what gets written!
fn draw(ctx: &cairo::Context, left: &str, Size { wd: width, ..}: Size) {
    // for the current time on the right
    let now = time::now();

    // the background is... gray-ish? this'll be configurable eventually
    ctx.set_source_rgb(0.1, 0.1, 0.1);
    ctx.paint();

    // and the text is white
    ctx.set_source_rgb(1.0, 1.0, 1.0);

    // Our pango layout handles placing and drawing text
    let layout = pangocairo::functions::create_layout(&ctx).unwrap();
    // for the time, we want to align it to the right
    layout.set_alignment(pango::Alignment::Right);
    // allow for the whole width of the bar, minus a small fixed amount
    layout.set_width((width - 20) * pango::SCALE);
    // this should also be configurable, but Fira Mono is a good font
    let mut font = pango::FontDescription::from_string("Fira Mono 18");
    font.set_weight(pango::Weight::Bold);
    layout.set_font_description(&font);
    // start drawing in the top-left
    ctx.move_to(10.0, 4.0);
    //The text here is just the nicely-rendered current time
    layout.set_text(&time::strftime("%a %b %d %H:%M", &now).unwrap());
    // and draw it
    pangocairo::functions::show_layout(&ctx, &layout);

    // We can reuse the same layout, but starting from the left
    layout.set_alignment(pango::Alignment::Left);
    // and now we write whatever the "current text" is...
    layout.set_text(left);
    // and draw that!
    pangocairo::functions::show_layout(&ctx, &layout);
}
