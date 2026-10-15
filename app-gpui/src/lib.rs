mod appearance;
pub mod ui;
mod window_icon;

#[cfg(all(feature = "ui-qualification", target_os = "linux"))]
pub mod qualification;
