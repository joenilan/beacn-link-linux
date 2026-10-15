//! Updates only this GPUI window's X11 icon. Wayland uses compositor/app identity.
#[cfg(target_os = "linux")]
pub fn update(window: &gpui_kit::Window, status: beacn_linx_core::icon::Status) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::ffi::{c_char, c_int, c_uchar, c_ulong, c_void};
    #[link(name = "X11")]
    unsafe extern "C" {
        fn XOpenDisplay(name: *const c_char) -> *mut c_void;
        fn XInternAtom(display: *mut c_void, name: *const c_char, only: c_int) -> c_ulong;
        fn XChangeProperty(
            display: *mut c_void,
            window: c_ulong,
            property: c_ulong,
            kind: c_ulong,
            format: c_int,
            mode: c_int,
            data: *const c_uchar,
            count: c_int,
        ) -> c_int;
        fn XCloseDisplay(display: *mut c_void) -> c_int;
    }
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let id = match handle.as_raw() {
        RawWindowHandle::Xlib(h) => h.window,
        RawWindowHandle::Xcb(h) => h.window.get() as c_ulong,
        _ => return,
    };
    let icon = beacn_linx_core::icon::raster(status, 64);
    let mut values = vec![icon.width as c_ulong, icon.height as c_ulong];
    values.extend(icon.rgba.chunks_exact(4).map(|p| {
        ((p[3] as c_ulong) << 24)
            | ((p[0] as c_ulong) << 16)
            | ((p[1] as c_ulong) << 8)
            | (p[2] as c_ulong)
    }));
    // The borrowed native handle remains alive on this window's render thread.
    // Xlib expects c_ulong elements even for a 32-bit CARDINAL property.
    unsafe {
        let display = XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return;
        }
        let property = XInternAtom(display, c"_NET_WM_ICON".as_ptr(), 0);
        let kind = XInternAtom(display, c"CARDINAL".as_ptr(), 0);
        XChangeProperty(
            display,
            id,
            property,
            kind,
            32,
            0,
            values.as_ptr().cast(),
            values.len() as c_int,
        );
        XCloseDisplay(display);
    }
}
#[cfg(not(target_os = "linux"))]
pub fn update(_: &gpui_kit::Window, _: beacn_linx_core::icon::Status) {}
