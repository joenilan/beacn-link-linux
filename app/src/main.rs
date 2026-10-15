use beacn_linx_app::{
    ui::{self, Interface},
    worker::{Connection, Worker},
};
use eframe::egui;
use std::{path::PathBuf, time::Duration};

struct App {
    interface: Interface,
    worker: Worker,
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let snapshot = self.worker.snapshot();
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
                .with_inner_size([1060., 760.])
                .with_min_inner_size([880., 600.]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            ui::style(&cc.egui_ctx);
            Ok(Box::new(App {
                interface: Interface::default(),
                worker: Worker::spawn(connection),
            }))
        }),
    )
}
