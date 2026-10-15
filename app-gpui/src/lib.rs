mod appearance;
mod console;
mod meter;
mod shutdown;
#[cfg(all(feature = "native-control", target_os = "linux"))]
pub mod control;
#[cfg(all(feature = "native-control", target_os = "linux"))]
mod frame;
#[cfg(all(feature = "ui-qualification", target_os = "linux"))]
pub mod diagnostics;
pub mod ui;
#[cfg(all(feature = "link", target_os = "linux"))]
pub mod link_ui;
mod window_icon;

#[cfg(all(feature = "ui-qualification", target_os = "linux"))]
pub mod qualification;
