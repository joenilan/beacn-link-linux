//! Test-feature-only scripted input inside an isolated eframe window. No OS input.
#[cfg(feature = "ui-qualification")]
fn main() -> eframe::Result {
    use beacn_linx_app::{
        ui::{self, Interface},
        worker::{Connection, Worker},
    };
    use eframe::egui::{self, Event, Modifiers, PointerButton};
    use std::{
        fs::OpenOptions,
        io::Write,
        path::PathBuf,
        time::{Duration, Instant},
    };
    struct Probe {
        ui: Interface,
        worker: Worker,
        root: PathBuf,
        stage: u8,
        input: Vec<Event>,
        first: String,
        second: String,
        start: Instant,
        shot: bool,
        failed: bool,
    }
    impl Probe {
        fn click(&mut self, key: &str) -> bool {
            let Some(rect) = self.ui.widgets.get(key) else {
                return false;
            };
            let pos = rect.center();
            self.input.extend([
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::NONE,
                },
            ]);
            true
        }
    }
    impl eframe::App for Probe {
        fn raw_input_hook(&mut self, _: &egui::Context, raw: &mut egui::RawInput) {
            raw.events.append(&mut self.input);
        }
        fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            let snapshot = self.worker.snapshot();
            for event in ctx.input(|i| i.raw.events.clone()) {
                if let Event::Screenshot { image, .. } = event {
                    let path = self.root.join("selected.ppm");
                    let mut file = OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(path)
                        .expect("exclusive evidence image");
                    write!(file, "P6\n{} {}\n255\n", image.size[0], image.size[1]).unwrap();
                    for pixel in &image.pixels {
                        file.write_all(&pixel.to_array()[..3]).unwrap();
                    }
                    self.shot = true;
                }
            }
            for command in self.ui.show(ctx, &snapshot) {
                if let Err(error) = self.worker.send(command, &snapshot) {
                    eprintln!("probe command refused: {error}");
                    self.failed = true;
                }
            }
            let first = beacn_link_protocol::planner::ApplicationId(self.first.clone());
            let second = beacn_link_protocol::planner::ApplicationId(self.second.clone());
            match self.stage {
                0 if snapshot.connected => {
                    if self.click(&format!("select:{}", self.first)) {
                        self.stage = 1;
                    }
                }
                1 if snapshot.selected.len() == 1 => {
                    if self.click(&format!("alias:{}", self.second)) {
                        self.stage = 2;
                    }
                }
                2 => {
                    self.input.extend([
                        Event::Key {
                            key: egui::Key::A,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: Modifiers::CTRL,
                        },
                        Event::Text("Other Player".into()),
                    ]);
                    self.stage = 3;
                }
                3 => {
                    self.stage = 4;
                }
                4 => {
                    if self.click(&format!("select:{}", self.second)) {
                        self.stage = 5;
                    }
                }
                5 if snapshot.selected.len() == 2 => {
                    if self.click(&format!("alias:{}", self.first)) {
                        self.stage = 6;
                    }
                }
                6 => {
                    self.input.extend([
                        Event::Key {
                            key: egui::Key::A,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: Modifiers::CTRL,
                        },
                        Event::Text("Media Player".into()),
                    ]);
                    self.stage = 7;
                }
                7 => {
                    self.stage = 8;
                }
                8 => {
                    if self.click(&format!("save:{}", self.first)) {
                        self.stage = 9;
                    }
                }
                9 if snapshot
                    .selected
                    .get(&first)
                    .is_some_and(|s| s == "Media Player") =>
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                    self.stage = 10;
                }
                10 if self.shot => {
                    if self.click(&format!("remove:{}", self.second)) {
                        self.stage = 11;
                    }
                }
                11 if snapshot.selected.len() == 1 && !snapshot.selected.contains_key(&second) => {
                    self.worker.stop();
                    let configs = std::fs::read_dir("/tmp")
                        .unwrap()
                        .filter_map(Result::ok)
                        .filter(|e| {
                            e.file_name()
                                .to_string_lossy()
                                .starts_with(&format!("beacn-linx-catalog-{}-", std::process::id()))
                        })
                        .count();
                    if configs != 0 {
                        self.failed = true;
                    }
                    let mut result = OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(self.root.join("result.txt"))
                        .unwrap();
                    writeln!(result,"widgets_selected=2 renamed=true removed=true worker_stopped=true owned_configs_remaining={configs} failed={}",self.failed).unwrap();
                    self.stage = 12;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                _ => {}
            }
            if self.start.elapsed() > Duration::from_secs(15) || self.failed {
                self.failed = true;
                self.worker.stop();
                let _ = std::fs::write(
                    self.root.join("failure.txt"),
                    format!(
                        "stage={} connected={} selected={} message={:?}",
                        self.stage,
                        snapshot.connected,
                        snapshot.selected.len(),
                        snapshot.message
                    ),
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 5 {
        eprintln!("qualification: interface_probe PRIVATE-RUNTIME CORE FIRST-ID SECOND-ID NEW-EVIDENCE-DIR");
        std::process::exit(2);
    }
    let root = PathBuf::from(&args[4]);
    std::fs::create_dir(&root).expect("exclusive evidence directory");
    let connection = Connection::Private {
        runtime: PathBuf::from(&args[0]),
        core: args[1].clone(),
    };
    eframe::run_native(
        "Beacn LinX · isolated interface qualification",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1060., 760.]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            ui::style(&cc.egui_ctx);
            Ok(Box::new(Probe {
                ui: Interface::default(),
                worker: Worker::spawn(connection),
                root,
                stage: 0,
                input: vec![],
                first: args[2].clone(),
                second: args[3].clone(),
                start: Instant::now(),
                shot: false,
                failed: false,
            }))
        }),
    )
}
#[cfg(not(feature = "ui-qualification"))]
fn main() {
    eprintln!("requires explicit ui-qualification feature");
    std::process::exit(2);
}
