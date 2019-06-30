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
    pub fn create() -> Result<Display, failure::Error> {
        let display = unsafe { xlib::XOpenDisplay(ptr::null()) };
        if display.is_null() {
            bail!("Unable to open X11 display");
        }
        let screen = unsafe { xlib::XDefaultScreen(display) };
        Ok(Display { display, screen })
    }

    pub fn get_width(&mut self) -> i32 {
        unsafe {
            let s = xlib::XScreenOfDisplay(self.display, self.screen);
            xlib::XWidthOfScreen(s)
        }
    }

    pub fn get_widths(&mut self) -> Result<Vec<(i32,i32)>, failure::Error> {
        if unsafe { x11::xinerama::XineramaIsActive(self.display) != 0 } {
            let mut screens = 0;
            let screen_info = unsafe { x11::xinerama::XineramaQueryScreens(self.display, &mut screens) };
            let mut widths = Vec::new();
            for i in 0..screens {
                unsafe {
                    let si = screen_info.offset(i as isize).as_ref().ok_or(format_err!("bad pointer"))?;
                    widths.push((si.x_org as i32, si.width as i32));
                }
            }
            Ok(widths)
        } else {
            Ok(vec![(0, self.get_width())])
        }
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe {
            xlib::XCloseDisplay(self.display);
        }
    }
}

/// All the state needed to keep around to run this sort of
/// application!
pub struct Window<'t> {
    pub display: &'t Display,
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

impl<'t> Window<'t> {
    /// Create a new Window from a given Display and with the desire
    /// width and height
    pub fn create(
        display: &'t Display,
        Size { wd: width, ht: height }: Size,
    ) -> Result<Window<'t>, failure::Error> {
        unsafe {
            let screen = display.screen;
            let window = xlib::XCreateSimpleWindow(
                display.display,
                xlib::XRootWindow(display.display, screen),
                0,
                0,
                width as u32,
                height as u32,
                1,
                xlib::XBlackPixel(display.display, screen),
                xlib::XWhitePixel(display.display, screen),
            );
            let wm_protocols = {
                let cstr = CString::new("WM_PROTOCOLS")?;
                xlib::XInternAtom(display.display, cstr.as_ptr(), 0)
            };
            let wm_delete_window = {
                let cstr = CString::new("WM_DELETE_WINDOW")?;
                xlib::XInternAtom(display.display, cstr.as_ptr(), 0)
            };
            Ok(Window {
                display,
                screen,
                window,
                wm_protocols,
                wm_delete_window,
                width,
                height,
            })
        }
    }

    /// for this application, we might eventually care about the
    /// mouse, so make sure we notify x11 that we care about those
    pub fn set_input_masks(&mut self) -> Result<(), failure::Error> {
        let mut opcode = 0;
        let mut event = 0;
        let mut error = 0;

        let xinput_str = CString::new("XInputExtension")?;
        unsafe {
            xlib::XQueryExtension(
                self.display.display,
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
                self.display.display,
                self.window,
                &mut input_event_mask,
                1,
            )
        } {
            status if status as u8 == xlib::Success => (),
            err => bail!("Failed to select events {:?}", err)
        }

        Ok(())
    }

    pub fn set_protocols(&mut self) -> Result<(), failure::Error> {
        let mut protocols = [self.intern("WM_DELETE_WINDOW")?];
        unsafe {
            xlib::XSetWMProtocols(
                self.display.display,
                self.window,
                protocols.as_mut_ptr(),
                protocols.len() as c_int,
            );
        }
        Ok(())
    }

    /// Set the name of the window to the desired string
    pub fn set_title(&mut self, name: &str) -> Result<(), failure::Error> {
        unsafe {
            xlib::XStoreName(
                self.display.display,
                self.window,
                CString::new(name)?.as_ptr(),
            );
        }
        Ok(())
    }

    /// Map the window to the screen
    pub fn map(&mut self) {
        unsafe {
            xlib::XMapWindow(self.display.display, self.window);
        }
    }

    /// Intern a string in the x server
    pub fn intern(&mut self, s: &str) -> Result<u64, failure::Error> {
        unsafe {
            let cstr = CString::new(s)?;
            Ok(xlib::XInternAtom(self.display.display, cstr.as_ptr(), 0))
        }
    }

    /// Modify the supplied property to the noted value.
    pub fn change_property<T: XProperty>(
        &mut self,
        prop: &str,
        val: &[T]
    ) -> Result<(), failure::Error>
    {
        let prop = self.intern(prop)?;
        unsafe {
            let len = val.len();
            T::with_ptr(val, self, |w, typ, ptr| {
                xlib::XChangeProperty(
                    w.display.display,
                    w.window,
                    prop,
                    typ,
                    32,
                    xlib::PropModeReplace,
                    ptr,
                    len as c_int,
                );
            })?;
        }
        Ok(())
    }

    /// Get the Cairo drawing surface corresponding to the whole
    /// window
    pub fn get_cairo_surface(&mut self) -> cairo::Surface {
        unsafe {
            let s = cairo_sys::cairo_xlib_surface_create(
                self.display.display,
                self.window,
                xlib::XDefaultVisual(self.display.display, self.screen),
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
        unsafe { xlib::XNextEvent(self.display.display, &mut e) };
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
                unsafe { xlib::XGetEventData(self.display.display, &mut cookie) };
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
            xlib::XPending(self.display.display) != 0
        }
    }

    /// Did you know that X11 uses a file descriptor underneath the
    /// surface to wait on events? This lets us use select on it!
    pub fn get_fd(&mut self) -> i32 {
        unsafe {
            xlib::XConnectionNumber(self.display.display)
        }
    }

    pub fn size(&self) -> Size {
        Size { wd: self.width, ht: self.height }
    }
}

/// A trait for abstracting over different values which are allowed
/// for xlib properties
pub trait XProperty : Sized {
    fn with_ptr(
        xs: &[Self],
        w: &mut Window,
        f: impl FnOnce(&mut Window, u64, *const u8),
    ) -> Result<(), failure::Error> ;
}

impl XProperty for i64 {
    fn with_ptr(
        xs: &[Self],
        w: &mut Window,
        f: impl FnOnce(&mut Window, u64, *const u8),
    ) -> Result<(), failure::Error> {
        f(w, xlib::XA_CARDINAL, unsafe { mem::transmute(xs.as_ptr()) });
        Ok(())
    }
}

impl XProperty for &str {
    fn with_ptr(
        xs: &[Self],
        w: &mut Window,
        f: impl FnOnce(&mut Window, u64, *const u8),
    ) -> Result<(), failure::Error> {
        let xs: Result<Vec<u64>, failure::Error> =
            xs.iter().map(|s| w.intern(s)).collect();
        f(w, xlib::XA_ATOM, unsafe { mem::transmute(xs?.as_ptr()) });
        Ok(())
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
