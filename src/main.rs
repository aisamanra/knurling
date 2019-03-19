mod window;

use x11::xlib;
use x11::xinput2;

use std::ffi::CString;
use std::os::raw::{c_int,c_uchar};
use std::ptr;

use pango::LayoutExt;

use window::{Event,Window};

fn main() {
    unsafe {
        let mut w = Window::create();
        w.change_property("_NET_WM_WINDOW_TYPE", "_NET_WM_WINDOW_TYPE_DOCK");

        {
            let prop = w.intern("_NET_WM_STRUT_PARTIAL");
            let val = [
                0i64, 0, 36, 0,
                0, 0, 0,  0,
                0, 3840, 0, 0,
            ];
            xlib::XChangeProperty(
                w.display,
                w.window,
                prop,
                xlib::XA_CARDINAL,
                32,
                xlib::PropModeReplace,
                std::mem::transmute(val.as_ptr()),
                val.len() as c_int,
            );
        }

        {
            let prop = w.intern("_NET_WM_STRUT");
            let val = &[
                0i64, 0, 36, 0,
            ];
            xlib::XChangeProperty(
                w.display,
                w.window,
                prop,
                xlib::XA_CARDINAL,
                32,
                xlib::PropModeReplace,
                std::mem::transmute(val.as_ptr()),
                val.len() as c_int,
            );
        }

        w.set_title("rbar");

        {
            let mut opcode = 0;
            let mut event = 0;
            let mut error = 0;
            let xinput_str = CString::new("XInputExtension").unwrap();
            let _xinput_available =
                xlib::XQueryExtension(w.display, xinput_str.as_ptr(), &mut opcode, &mut event, &mut error);

            let mut mask: [c_uchar;1] = [0];
            let mut input_event_mask = xinput2::XIEventMask {
                deviceid: xinput2::XIAllMasterDevices,
                mask_len: mask.len() as i32,
                mask: mask.as_mut_ptr(),
            };
            let events = &[
                xinput2::XI_ButtonPress,
                xinput2::XI_ButtonRelease,
            ];
            for &event in events {
                xinput2::XISetMask(&mut mask, event);
            }

        
            match xinput2::XISelectEvents(w.display, w.window, &mut input_event_mask, 1) {
                status if status as u8 == xlib::Success => (),
                err => panic!("Failed to select events {:?}", err)
            }
        }

        w.set_protocols();        
        w.map();

        let surf = w.get_cairo_surface();
        let ctx = cairo::Context::new(&surf);

        let window_fd = w.get_fd();

        let mut fds = std::mem::uninitialized();
        let mut input = format!("Loading...");
        let mut stdin = std::io::BufReader::new(std::io::stdin());
        let mut timer = libc::timeval {
            tv_sec: 5,
            tv_usec: 0,
        };
        draw(&ctx, "[1]");
        
        loop {
            use std::io::BufRead;

            libc::FD_ZERO(&mut fds);
            libc::FD_SET(window_fd, &mut fds);
            libc::FD_SET(1, &mut fds);

            libc::select(window_fd + 1, &mut fds, ptr::null_mut(), ptr::null_mut(), &mut timer);

            if libc::FD_ISSET(1, &mut fds) {
                input = String::new();
                stdin.read_line(&mut input).unwrap();
                if input == "" {
                    break;
                }
                draw(&ctx, &input);
            }

            while w.has_events() {
                draw(&ctx, &input);
                match w.handle() {
                    Event::QuitEvent => break,
                    e => (),
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
