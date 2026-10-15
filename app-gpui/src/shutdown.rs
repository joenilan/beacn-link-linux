//! Translate process termination into the same joined worker cleanup as Quit.
use std::sync::atomic::{AtomicBool, Ordering};

static REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn request(_: i32) {
    // Lock-free flag only; cleanup must run on the ordinary application thread.
    REQUESTED.store(true, Ordering::Relaxed);
}

pub fn install() {
    #[cfg(target_os = "linux")]
    unsafe {
        unsafe extern "C" {
            fn signal(number: i32, handler: usize) -> usize;
        }
        signal(15, request as *const () as usize);
        signal(2, request as *const () as usize);
    }
}

pub fn requested() -> bool {
    REQUESTED.load(Ordering::Relaxed)
}
