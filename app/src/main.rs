use beacn_linx_app::{
    ui::{self, Interface},
    worker::{Connection, Worker},
};
use eframe::egui;
use std::{path::PathBuf, time::Duration};

struct App {
    interface: Interface,
    worker: Worker,
    #[cfg(all(feature = "status-tray", target_os = "linux"))]
    tray: beacn_linx_app::tray::Tray,
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let snapshot = self.worker.snapshot();
        #[cfg(all(feature = "status-tray", target_os = "linux"))]
        {
            use beacn_linx_app::{model::Command, tray};
            self.tray.update(
                beacn_linx_app::icon::Status::from_snapshot(&snapshot),
                snapshot.link.phase == beacn_link_protocol::link_session::Phase::Running,
            );
            let actions = self.tray.actions();
            if actions & tray::SHOW != 0 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            if actions & tray::STOP != 0 {
                self.interface.feedback = self.worker.send(Command::Stop, &snapshot).err();
            }
            if actions & tray::QUIT != 0 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            self.interface.tray_unavailable = self.tray.available() == Some(false);
        }
        for command in self.interface.show(ctx, &snapshot) {
            self.interface.feedback = self.worker.send(command, &snapshot).err();
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.worker.stop();
    }
    fn persist_egui_memory(&self) -> bool {
        false
    }
}
fn main() -> eframe::Result {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--help"] {
        println!("Beacn LinX · Rust application preview\nDefault: read-only desktop application discovery\nOptional isolated test core: --private-runtime PATH --core NAME\nNo USB, route application, persistence or automatic reconnect.");
        return Ok(());
    }
    let connection = match args.as_slice() {
        [] => Connection::Desktop,
        [a, r, b, c] if a == "--private-runtime" && b == "--core" => Connection::Private {
            runtime: PathBuf::from(r),
            core: c.clone(),
        },
        _ => {
            eprintln!("Unknown arguments. Use --help.");
            std::process::exit(2);
        }
    };
    eframe::run_native(
        "Beacn LinX",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_icon(beacn_linx_app::icon::raster(
                    beacn_linx_app::icon::Status::Disconnected,
                    64,
                ))
                .with_inner_size([1060., 760.])
                .with_min_inner_size([880., 600.]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            ui::style(&cc.egui_ctx);
            Ok(Box::new(App {
                interface: Interface::default(),
                #[cfg(all(feature = "status-tray", target_os = "linux"))]
                tray: beacn_linx_app::tray::Tray::spawn(),
                worker: Worker::spawn(connection),
            }))
        }),
    )
}
