//! Optional Linux StatusNotifierItem. Bus work never runs on the render thread.
use crate::icon::{self, Status};
use ksni::blocking::TrayMethods;
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc, Mutex,
};

pub const SHOW: u8 = 1;
pub const STOP: u8 = 2;
pub const QUIT: u8 = 4;
/// A sound output was picked; read it with [`Tray::take_output`].
pub const OUTPUT: u8 = 8;
#[derive(Clone, PartialEq, Eq)]
struct View {
    status: Status,
    running: bool,
    /// Desktop outputs as (node.name, description), and the default's name.
    outputs: Vec<(String, String)>,
    default_output: Option<String>,
}
struct Item {
    view: View,
    actions: Arc<AtomicU8>,
    /// Output picked from the "Sound output" submenu.
    chosen: Arc<Mutex<Option<String>>>,
    /// xdg-activation token from the tray host for the latest click.
    token: Arc<Mutex<Option<String>>>,
}
impl ksni::Tray for Item {
    fn provide_xdg_activation_token(&mut self, token: String) {
        *self.token.lock().unwrap() = Some(token);
    }
    fn id(&self) -> String {
        "beacn-linx".into()
    }
    fn title(&self) -> String {
        format!("LinX · {}", self.view.status.label())
    }
    fn status(&self) -> ksni::Status {
        if self.view.status == Status::Error {
            ksni::Status::NeedsAttention
        } else {
            ksni::Status::Active
        }
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        [16, 22, 32, 64]
            .into_iter()
            .map(|size| {
                let image = icon::raster(self.view.status, size);
                let data = image
                    .rgba
                    .chunks_exact(4)
                    .flat_map(|p| [p[3], p[0], p[1], p[2]])
                    .collect();
                ksni::Icon {
                    width: size as i32,
                    height: size as i32,
                    data,
                }
            })
            .collect()
    }
    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "LinX".into(),
            description: self.view.status.label().into(),
            ..Default::default()
        }
    }
    fn attention_icon_pixmap(&self) -> Vec<ksni::Icon> {
        self.icon_pixmap()
    }
    fn activate(&mut self, _: i32, _: i32) {
        self.actions.fetch_or(SHOW, Ordering::Release);
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::{RadioGroup, RadioItem, StandardItem, SubMenu};
        let mut items: Vec<ksni::MenuItem<Self>> = vec![
            StandardItem {
                label: self.view.status.label().into(),
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Open LinX".into(),
                activate: Box::new(|item: &mut Self| {
                    item.actions.fetch_or(SHOW, Ordering::Release);
                }),
                ..Default::default()
            }
            .into(),
        ];
        if !self.view.outputs.is_empty() {
            let names: Vec<String> = self.view.outputs.iter().map(|(n, _)| n.clone()).collect();
            let selected = self
                .view
                .default_output
                .as_ref()
                .and_then(|d| names.iter().position(|n| n == d))
                .unwrap_or(usize::MAX);
            items.push(
                SubMenu {
                    label: "Sound output".into(),
                    submenu: vec![RadioGroup {
                        selected,
                        select: Box::new(move |item: &mut Self, index| {
                            if let Some(name) = names.get(index) {
                                *item.chosen.lock().unwrap() = Some(name.clone());
                                item.actions.fetch_or(OUTPUT, Ordering::Release);
                            }
                        }),
                        options: self
                            .view
                            .outputs
                            .iter()
                            .map(|(_, description)| RadioItem {
                                label: description.replace('_', "__"),
                                ..Default::default()
                            })
                            .collect(),
                    }
                    .into()],
                    ..Default::default()
                }
                .into(),
            );
        }
        items.extend([
            StandardItem {
                label: "Stop link".into(),
                enabled: self.view.running,
                activate: Box::new(|item: &mut Self| {
                    item.actions.fetch_or(STOP, Ordering::Release);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|item: &mut Self| {
                    item.actions.fetch_or(QUIT, Ordering::Release);
                }),
                ..Default::default()
            }
            .into(),
        ]);
        items
    }
}
pub struct Tray {
    view: Arc<Mutex<Option<View>>>,
    actions: Arc<AtomicU8>,
    token: Arc<Mutex<Option<String>>>,
    chosen: Arc<Mutex<Option<String>>>,
    availability: Arc<Mutex<Option<bool>>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Default for Tray {
    fn default() -> Self {
        Self::spawn()
    }
}
impl Tray {
    pub fn spawn() -> Self {
        let initial = View {
            status: Status::Disconnected,
            running: false,
            outputs: Vec::new(),
            default_output: None,
        };
        let view = Arc::new(Mutex::new(Some(initial.clone())));
        let actions = Arc::new(AtomicU8::new(0));
        let availability = Arc::new(Mutex::new(None));
        let token = Arc::new(Mutex::new(None));
        let chosen = Arc::new(Mutex::new(None));
        let (state, flags, ready, click, pick) = (
            view.clone(),
            actions.clone(),
            availability.clone(),
            token.clone(),
            chosen.clone(),
        );
        let thread = std::thread::spawn(move || {
            let handle = match (Item {
                view: initial.clone(),
                actions: flags,
                token: click,
                chosen: pick,
            })
            .spawn()
            {
                Ok(handle) => handle,
                Err(_) => {
                    *ready.lock().unwrap() = Some(false);
                    return;
                }
            };
            *ready.lock().unwrap() = Some(true);
            let mut previous = initial;
            loop {
                let current = state.lock().unwrap().clone();
                let Some(current) = current else {
                    break;
                };
                if handle.is_closed() {
                    *ready.lock().unwrap() = Some(false);
                    break;
                }
                if previous != current {
                    let next = current.clone();
                    handle.update(|item| item.view = next);
                    previous = current;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            handle.shutdown().wait();
        });
        Self {
            view,
            actions,
            token,
            chosen,
            availability,
            thread: Some(thread),
        }
    }
    pub fn update(&self, status: Status, running: bool) {
        if let Some(view) = self.view.lock().unwrap().as_mut() {
            view.status = status;
            view.running = running;
        }
    }
    /// Desktop outputs for the "Sound output" submenu, as (node.name,
    /// description), and the current default's name. Empty hides the menu.
    pub fn set_outputs(&self, outputs: Vec<(String, String)>, default_output: Option<String>) {
        if let Some(view) = self.view.lock().unwrap().as_mut() {
            view.outputs = outputs;
            view.default_output = default_output;
        }
    }
    /// The output picked from the submenu since the last call.
    pub fn take_output(&self) -> Option<String> {
        self.chosen.lock().unwrap().take()
    }
    pub fn actions(&self) -> u8 {
        self.actions.swap(0, Ordering::AcqRel)
    }
    /// Activation token the tray host supplied with the latest click, if any
    /// (KDE Plasma; lets the window be raised on Wayland).
    pub fn take_activation_token(&self) -> Option<String> {
        self.token.lock().unwrap().take()
    }
    pub fn available(&self) -> Option<bool> {
        *self.availability.lock().unwrap()
    }
}
impl Drop for Tray {
    fn drop(&mut self) {
        *self.view.lock().unwrap() = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
