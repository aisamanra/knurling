use x11::{xlib,xinput2};

use std::ffi::CString;
use std::{mem,ptr};
use std::os::raw::{c_int,c_uchar};

#[derive(Debug,Clone,Copy)]
pub struct Size {
    pub wd: i32,
    pub ht: i32,
}

pub struct Display {
    pub display: *mut xlib::_XDisplay,
    pub screen: i32,
}

impl Display {
    pub fn create() -> Display {
        let display = unsafe { xlib::XOpenDisplay(ptr::null()) };
        let screen = unsafe { xlib::XDefaultScreen(display) };
        Display { display, screen }
    }

    pub fn get_width(&mut self) -> i32 {
        unsafe {
            let s = xlib::XScreenOfDisplay(self.display, self.screen);
            xlib::XWidthOfScreen(s)
        }
    }
}

/// All the state needed to keep around to run this sort of
/// application!
pub struct Window {
    pub display: *mut xlib::_XDisplay,
    pub screen: i32,
    pub window: u64,
    // these two are interned strings kept around because we want to
    // check against them a _lot_, to find out if an event is a quit
    // event
    pub wm_protocols: u64,
    pub wm_delete_window: u64,
    // The width and height of the window
    pub width: i32,
    pub height: i32,
}

impl Window {
    /// Create a new Window from a given Display and with the desire
    /// width and height
    pub fn create(
        d: Display,
        Size { wd: width, ht: height }: Size,
    ) -> Window {
        unsafe {
            let display = d.display;
            let screen = d.screen;
            let window = xlib::XCreateSimpleWindow(
                display,
                xlib::XRootWindow(display, screen),
                0,
                0,
                width as u32,
                height as u32,
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
                width,
                height,
            }
        }
    }

    /// for this application, we might eventually care about the
    /// mouse, so make sure we notify x11 that we care about those
    pub fn set_input_masks(&mut self) {
        let mut opcode = 0;
        let mut event = 0;
        let mut error = 0;

        let xinput_str = CString::new("XInputExtension").unwrap();
        unsafe {
            xlib::XQueryExtension(
                self.display,
                xinput_str.as_ptr(),
                &mut opcode,
                &mut event,
                &mut error,
            );
        }

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

        match unsafe {
            xinput2::XISelectEvents(
                self.display,
                self.window,
                &mut input_event_mask,
                1,
            )
        } {
            status if status as u8 == xlib::Success => (),
            err => panic!("Failed to select events {:?}", err)
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

    /// Set the name of the window to the desired string
    pub fn set_title(&mut self, name: &str) {
        unsafe {
            xlib::XStoreName(
                self.display,
                self.window,
                CString::new(name).unwrap().as_ptr(),
            );
        }
    }

    /// Map the window to the screen
    pub fn map(&mut self) {
        unsafe {
            xlib::XMapWindow(self.display, self.window);
        }
    }

    /// Intern a string in the x server
    pub fn intern(&mut self, s: &str) -> u64 {
        unsafe {
            let cstr = CString::new(s).unwrap();
            xlib::XInternAtom(self.display, cstr.as_ptr(), 0)
        }
    }

    /// Modify the supplied property to the noted value.
    pub fn change_property<T: XProperty>(&mut self, prop: &str, val: &[T]) {
        let prop = self.intern(prop);
        unsafe {
            let len = val.len();
            T::with_ptr(val, self, |w, typ, ptr| {
                xlib::XChangeProperty(
                    w.display,
                    w.window,
                    prop,
                    typ,
                    32,
                    xlib::PropModeReplace,
                    ptr,
                    len as c_int,
                );
            });
        }
    }

    /// Get the Cairo drawing surface corresponding to the whole
    /// window
    pub fn get_cairo_surface(&mut self) -> cairo::Surface {
        unsafe {
            let s = cairo_sys::cairo_xlib_surface_create(
                self.display,
                self.window,
                xlib::XDefaultVisual(self.display, self.screen),
                self.width,
                self.height,
            );
            cairo::Surface::from_raw_none(s)
        }
    }

    /// handle a single event, wrapping it as an 'Event'. This is
    /// pretty useless right now, but the plan is to make it easier to
    /// handle things like keyboard input and mouse input later. This
    /// will also only return values for events we care about
    pub fn handle(&mut self) -> Option<Event> {
        let mut e = unsafe { mem::uninitialized() };
        unsafe { xlib::XNextEvent(self.display, &mut e) };
        match e.get_type() {
            // Is it a quit event? We gotta do some tedious string
            // comparison to find out
            xlib::ClientMessage => {
                let xclient: xlib::XClientMessageEvent = From::from(e);
                if xclient.message_type == self.wm_protocols && xclient.format == 32 {
                    let protocol = xclient.data.get_long(0) as xlib::Atom;
                    if protocol == self.wm_delete_window {
                        return Some(Event::QuitEvent);
                    }
                }
            }

            // Is it a show event?
            xlib::Expose => return Some(Event::ShowEvent),

            // otherwise, it might be a mouse press event
            xlib::GenericEvent => {
                let mut cookie: xlib::XGenericEventCookie = From::from(e);
                unsafe { xlib::XGetEventData(self.display, &mut cookie) };
                    match cookie.evtype {
                        xinput2::XI_ButtonPress => {
                            let data: &xinput2::XIDeviceEvent =
                                unsafe { mem::transmute(cookie.data) };
                            return Some(Event::MouseEvent { x: data.event_x, y: data.event_y });
                        }
                        _ => (),
                    }
            }
            _ => (),
        }

        None
    }

    /// True if there are any pending events.
    pub fn has_events(&mut self) -> bool {
        unsafe {
            xlib::XPending(self.display) != 0
        }
    }

    /// Did you know that X11 uses a file descriptor underneath the
    /// surface to wait on events? This lets us use select on it!
    pub fn get_fd(&mut self) -> i32 {
        unsafe {
            xlib::XConnectionNumber(self.display)
        }
    }
}

/// Always close the display when we're done.
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            xlib::XCloseDisplay(self.display);
        }
    }
}

/// A trait for abstracting over different values which are allowed
/// for xlib properties
pub trait XProperty : Sized {
    fn with_ptr(
        xs: &[Self],
        w: &mut Window,
        f: impl FnOnce(&mut Window, u64, *const u8),
    );
}

impl XProperty for i64 {
    fn with_ptr(
        xs: &[Self],
        w: &mut Window,
        f: impl FnOnce(&mut Window, u64, *const u8),
    ) {
        f(w, xlib::XA_CARDINAL, unsafe { mem::transmute(xs.as_ptr()) })
    }
}

impl XProperty for &str {
    fn with_ptr(
        xs: &[Self],
        w: &mut Window,
        f: impl FnOnce(&mut Window, u64, *const u8),
    ) {
        let xs: Vec<u64> = xs.iter().map(|s| w.intern(s)).collect();
        f(w, xlib::XA_ATOM, unsafe { mem::transmute(xs.as_ptr()) })
    }
}

/// An ADT of only the events we care about, wrapped in a high-level
/// way
#[derive(Debug)]
pub enum Event {
    MouseEvent { x:f64, y: f64 },
    ShowEvent,
    QuitEvent,
}
