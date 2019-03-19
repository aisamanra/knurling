use x11::{xlib,xinput2};

use std::ffi::CString;
use std::{mem,ptr};
use std::os::raw::c_int;

pub struct Window {
    pub display: *mut xlib::_XDisplay,
    pub screen: i32,
    pub window: u64,
    pub wm_protocols: u64,
    pub wm_delete_window: u64,
}

impl Window {
    pub fn create() -> Window {
        unsafe {
            let display = xlib::XOpenDisplay(ptr::null());
            let screen = xlib::XDefaultScreen(display);
            let window = xlib::XCreateSimpleWindow(
                display,
                xlib::XRootWindow(display, screen),
                0,
                0,
                3840,
                36,
                1,
                xlib::XBlackPixel(display, screen),
                xlib::XWhitePixel(display, screen),
            );
            let wm_protocols = {
                let cstr = CString::new("WM_PROTOCOLS").unwrap();
                xlib::XInternAtom(display, cstr.as_ptr(), 0)
            };
            let wm_delete_window = {
                let cstr = CString::new("WM_DELETE_WINDOW").unwrap();
                xlib::XInternAtom(display, cstr.as_ptr(), 0)
            };
            Window {
                display,
                screen,
                window,
                wm_protocols,
                wm_delete_window,
            }
        }
    }

    pub fn set_protocols(&mut self) {
        let mut protocols = [self.intern("WM_DELETE_WINDOW")];
        unsafe {
            xlib::XSetWMProtocols(
                self.display,
                self.window,
                protocols.as_mut_ptr(),
                protocols.len() as c_int,
            );
        }
    }

    pub fn set_title(&mut self, name: &str) {
        unsafe {
            xlib::XStoreName(
                self.display,
                self.window,
                CString::new(name).unwrap().as_ptr(),
            );
        }
    }

    pub fn map(&mut self) {
        unsafe {
            xlib::XMapWindow(self.display, self.window);
        }
    }

    pub fn intern(&mut self, s: &str) -> u64 {
        unsafe {
            let cstr = CString::new(s).unwrap();
            xlib::XInternAtom(self.display, cstr.as_ptr(), 0)
        }
    }

    pub fn change_property(&mut self, prop: &str, val: &str) {
        let prop = self.intern(prop);
        let val  = self.intern(val);
        unsafe {
            xlib::XChangeProperty(
                self.display,
                self.window,
                prop,
                xlib::XA_ATOM,
                32,
                xlib::PropModeReplace,
                mem::transmute(&val),
                1,
            );
        }
    }

    pub fn get_cairo_surface(&mut self) -> cairo::Surface {
        unsafe {
            let s = cairo_sys::cairo_xlib_surface_create(
                self.display,
                self.window,
                xlib::XDefaultVisual(self.display, self.screen),
                3840,
                64,
            );
            cairo::Surface::from_raw_none(s)
        }
}

    pub fn handle(&mut self) -> Event {
        // to find out if we're getting a delete window event

        let mut e = unsafe { mem::uninitialized() };
        unsafe { xlib::XNextEvent(self.display, &mut e) };
        match e.get_type() {
            xlib::ClientMessage => {
                let xclient: xlib::XClientMessageEvent = From::from(e);
                if xclient.message_type == self.wm_protocols && xclient.format == 32 {
                    let protocol = xclient.data.get_long(0) as xlib::Atom;
                    if protocol == self.wm_delete_window {
                        return Event::QuitEvent;
                    }
                }
            }

            xlib::Expose => return Event::ShowEvent,
                
            xlib::GenericEvent => {
                let mut cookie: xlib::XGenericEventCookie = From::from(e);
                unsafe { xlib::XGetEventData(self.display, &mut cookie) };
                    match cookie.evtype {
                        xinput2::XI_ButtonPress => {
                            let data: &xinput2::XIDeviceEvent = unsafe { mem::transmute(cookie.data) };
                            return Event::MouseEvent { x: data.event_x, y: data.event_y };
                        }
                        _ => (),
                    }
            }
            _ => (),
        }

        Event::Other
    }

    pub fn has_events(&mut self) -> bool {
        unsafe {
            xlib::XPending(self.display) != 0
        }
    }

    pub fn show(&mut self) {
        unsafe {
            xlib::XClearArea(
                self.display,
                self.window,
                0,
                0,
                3840,
                36,
                1
            );
        }
    }

    pub fn get_fd(&mut self) -> i32 {
        unsafe {
            xlib::XConnectionNumber(self.display)
        }
    }
}


impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            xlib::XCloseDisplay(self.display);
        }
    }
}

#[derive(Debug)]
pub enum Event {
    MouseEvent { x:f64, y: f64 },
    ShowEvent,
    QuitEvent,
    Other,
}

/*
cairo_surface_t *cairo_create_x11_surface0(int x, int y)
{
    Display *dsp;
    Drawable da;
    int screen;
    cairo_surface_t *sfc;

    if ((dsp = XOpenDisplay(NULL)) == NULL)
        exit(1);
    screen = DefaultScreen(dsp);
    da = XCreateSimpleWindow(dsp, DefaultRootWindow(dsp),
        0, 0, x, y, 0, 0, 0);
    XSelectInput(dsp, da, ButtonPressMask | KeyPressMask);
    XMapWindow(dsp, da);

    sfc = cairo_xlib_surface_create(dsp, da,
        DefaultVisual(dsp, screen), x, y);
    cairo_xlib_surface_set_size(sfc, x, y);

    return sfc;
}
 */
