//! Capture only the native window handle supplied by GPUI. No desktop/input API.
use gpui_kit::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void};

#[link(name = "X11")]
unsafe extern "C" {
    fn XOpenDisplay(name: *const c_char) -> *mut c_void;
    fn XGetGeometry(
        d: *mut c_void,
        w: c_ulong,
        root: *mut c_ulong,
        x: *mut c_int,
        y: *mut c_int,
        width: *mut c_uint,
        height: *mut c_uint,
        border: *mut c_uint,
        depth: *mut c_uint,
    ) -> c_int;
    fn XGetImage(
        d: *mut c_void,
        w: c_ulong,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
        mask: c_ulong,
        format: c_int,
    ) -> *mut c_void;
    fn XGetPixel(image: *mut c_void, x: c_int, y: c_int) -> c_ulong;
    fn XDestroyImage(image: *mut c_void) -> c_int;
    fn XCloseDisplay(d: *mut c_void) -> c_int;
}

pub fn ppm(window: &Window) -> Result<Vec<u8>, String> {
    if let Ok(image) = window.render_to_image() {
        let mut bytes = format!("P6\n{} {}\n255\n", image.width(), image.height()).into_bytes();
        for p in image.pixels() {
            bytes.extend_from_slice(&p.0[..3]);
        }
        return Ok(bytes);
    }
    let id = match HasWindowHandle::window_handle(window)
        .map_err(|e| e.to_string())?
        .as_raw()
    {
        RawWindowHandle::Xlib(h) => h.window,
        RawWindowHandle::Xcb(h) => h.window.get() as c_ulong,
        _ => return Err("Own-window capture needs GPU readback or X11/Xwayland.".into()),
    };
    unsafe {
        let d = XOpenDisplay(std::ptr::null());
        if d.is_null() {
            return Err("Own-window display unavailable.".into());
        }
        struct Display(*mut c_void);
        impl Drop for Display {
            fn drop(&mut self) {
                unsafe {
                    XCloseDisplay(self.0);
                }
            }
        }
        let _display = Display(d);
        let (mut root, mut x, mut y, mut w, mut h, mut border, mut depth) = (0, 0, 0, 0, 0, 0, 0);
        if XGetGeometry(
            d,
            id,
            &mut root,
            &mut x,
            &mut y,
            &mut w,
            &mut h,
            &mut border,
            &mut depth,
        ) == 0
            || w == 0
            || h == 0
            || w > 2560
            || h > 1600
            || ![24, 32].contains(&depth)
        {
            return Err("Own-window geometry unavailable or exceeds capture bound.".into());
        }
        let image = XGetImage(d, id, 0, 0, w, h, !0, 2);
        if image.is_null() {
            return Err("Own-window pixels unavailable.".into());
        }
        let mut bytes = format!("P6\n{w} {h}\n255\n").into_bytes();
        for row in 0..h {
            for col in 0..w {
                let p = XGetPixel(image, col as c_int, row as c_int);
                bytes.extend([(p >> 16) as u8, (p >> 8) as u8, p as u8]);
            }
        }
        XDestroyImage(image);
        Ok(bytes)
    }
}
