pub use beacn_linx_core::icon::Status;
pub fn color(status: Status) -> eframe::egui::Color32 {
    let [r, g, b, _] = status.color();
    eframe::egui::Color32::from_rgb(r, g, b)
}
pub fn raster(status: Status, size: u32) -> eframe::egui::IconData {
    let icon = beacn_linx_core::icon::raster(status, size);
    eframe::egui::IconData {
        rgba: icon.rgba,
        width: icon.width,
        height: icon.height,
    }
}
