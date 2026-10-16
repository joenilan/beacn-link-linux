pub mod autostart;
pub mod continuity;
pub mod icon;
#[cfg(all(feature = "link", target_os = "linux"))]
pub mod link;
pub mod model;
pub mod preferences;
#[cfg(all(feature = "private-session-test", target_os = "linux"))]
pub mod private_session;
#[cfg(all(feature = "private-session-test", target_os = "linux"))]
mod session_owner;
pub mod startup;
#[cfg(all(feature = "studio-session-test", target_os = "linux"))]
pub mod studio_session;
#[cfg(all(feature = "status-tray", target_os = "linux"))]
pub mod tray;
pub mod worker;

#[cfg(all(feature = "desktop-session-test", target_os = "linux"))]
pub mod desktop_session;
