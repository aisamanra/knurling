mod window;

use std::ptr;
use std::io::Write;
use std::os::unix::io::AsRawFd;

use pango::LayoutExt;

use window::{Event,Window};

fn main() {
    unsafe {
        let mut w = Window::create();
        w.change_property(
            "_NET_WM_WINDOW_TYPE",
            &["_NET_WM_WINDOW_TYPE_DOCK"],
        );

        w.change_property(
            "_NET_WM_STRUT_PARTIAL",
            &[
                0,    0, 36,  0,
                0,    0,  0,  0,
                0, 3840,  0,  0i64,
            ],
        );
        w.change_property(
            "_NET_WM_STRUT",
            &[0i64, 0, 36, 0],
        );

        w.set_title("rbar");

        w.set_input_masks();

        w.set_protocols();
        w.map();

        let surf = w.get_cairo_surface();
        let ctx = cairo::Context::new(&surf);

        let window_fd = w.get_fd();

        let mut fds = std::mem::uninitialized();
        let mut input = format!("Loading...");
        let stdin_fd = std::io::stdin().as_raw_fd();
        let mut stdin = std::io::BufReader::new(std::io::stdin());
        let mut timer = libc::timeval {
            tv_sec: 5,
            tv_usec: 0,
        };
        let mut log = std::fs::File::create("/home/gdritter/log.txt").unwrap();
        draw(&ctx, "[1]");

        loop {
            use std::io::BufRead;

            libc::FD_ZERO(&mut fds);
            libc::FD_SET(window_fd, &mut fds);
            libc::FD_SET(stdin_fd, &mut fds);

            libc::select(window_fd + 1, &mut fds, ptr::null_mut(), ptr::null_mut(), &mut timer);

            if libc::FD_ISSET(stdin_fd, &mut fds) {
                input = String::new();
                stdin.read_line(&mut input).unwrap();
                log.write_fmt(format_args!("got {}", input)).unwrap();
                if input == "" {
                    break;
                }
                draw(&ctx, &input);
            }

            while w.has_events() {
                draw(&ctx, &input);
                match w.handle() {
                    Event::QuitEvent => break,
                    Event::ShowEvent =>
                        draw(&ctx, &input),
                    _e => (),
                }
            }

        }
    }
}


fn draw(ctx: &cairo::Context, left: &str) {
    let now = time::now();

    ctx.set_source_rgb(0.1, 0.1, 0.1);
    ctx.paint();
    ctx.set_source_rgb(1.0, 1.0, 1.0);

    let layout = pangocairo::functions::create_layout(&ctx).unwrap();
    layout.set_alignment(pango::Alignment::Right);
    layout.set_width((3840 - 20) * pango::SCALE);
    let mut font = pango::FontDescription::from_string("Fira Mono 18");
    font.set_weight(pango::Weight::Bold);
    layout.set_font_description(&font);
    ctx.move_to(10.0, 4.0);
    layout.set_text(&time::strftime("%a %b %d %H:%M", &now).unwrap());
    pangocairo::functions::show_layout(&ctx, &layout);

    layout.set_alignment(pango::Alignment::Left);
    layout.set_text(left);
    pangocairo::functions::show_layout(&ctx, &layout);
}
