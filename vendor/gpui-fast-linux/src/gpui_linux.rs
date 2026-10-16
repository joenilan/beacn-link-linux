// Modified by Longbridge for gpui-fast.
#![cfg(any(target_os = "linux", target_os = "freebsd"))]
mod fast;
mod linux;

pub use linux::{current_platform, linux_platform};

/// LinX patch: an xdg-activation token received after startup (for example
/// from the tray's StatusNotifierItem click). The next new toplevel window,
/// or the next `activate` of an existing one, uses it so the compositor
/// raises and focuses the window instead of only marking it urgent.
pub fn set_pending_activation_token(token: String) {
    if let Ok(mut pending) = linux::PENDING_ACTIVATION_TOKEN.lock() {
        *pending = Some(token).filter(|t| !t.is_empty());
    }
}
