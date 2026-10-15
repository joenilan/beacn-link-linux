use beacn_link_protocol::{link_session::Phase, planner::ApplicationId};
use beacn_linx_core::{
    icon::{self, Status},
    model::{self, Command, Snapshot},
    worker::{Connection, Worker},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{
        Disableable, IconName, Sizable, Theme, ThemeMode,
        button::{Button, ButtonVariants},
        input::{Input, InputEvent, InputState},
    },
    *,
};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

const BG: u32 = 0x111318;
const PANEL: u32 = 0x191c23;
const LINE: u32 = 0x30343e;
const TEXT: u32 = 0xebeef5;
const MUTED: u32 = 0x969eae;
const TEAL: u32 = 0x5dd9bb;

fn color(status: Status) -> Rgba {
    let [r, g, b, _] = status.color();
    rgb((r as u32) << 16 | (g as u32) << 8 | b as u32)
}
fn icon_path(status: Status) -> &'static str {
    match status {
        Status::Disconnected => "lynx/gray.svg",
        Status::Connecting => "lynx/amber.svg",
        Status::Connected => "lynx/teal.svg",
        Status::Error => "lynx/red.svg",
        Status::PrivateTest => "lynx/blue.svg",
    }
}
struct Assets;
impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        for status in [
            Status::Disconnected,
            Status::Connecting,
            Status::Connected,
            Status::Error,
            Status::PrivateTest,
        ] {
            if path == icon_path(status) {
                return Ok(Some(Cow::Owned(icon::svg(status).into_bytes())));
            }
        }
        assets::Assets.load(path)
    }
    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        assets::Assets.list(path)
    }
}
pub struct Interface {
    native_icon: Option<Status>,
    worker: Worker,
    snapshot: Snapshot,
    search: Entity<InputState>,
    names: BTreeMap<ApplicationId, Entity<InputState>>,
    expanded: BTreeSet<ApplicationId>,
    channel: Option<u8>,
    settings: bool,
    tray_available: Option<bool>,
    feedback: Option<String>,
    _poll: Task<()>,
    #[cfg(all(feature = "ui-qualification", target_os = "linux"))]
    probe: Option<crate::qualification::Probe>,
    #[cfg(all(feature = "status-tray", target_os = "linux"))]
    tray: beacn_linx_core::tray::Tray,
}
impl Interface {
    pub fn new(connection: Connection, window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.on_app_quit(|this, _| {
            this.worker.stop();
            #[cfg(all(feature = "ui-qualification", target_os = "linux"))]
            if let Some(probe) = &this.probe {
                probe.on_quit(&this.worker.snapshot());
            }
            async {}
        })
        .detach();
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Find an application"));
        cx.subscribe_in(&search, window, |_, _, _: &InputEvent, _, cx| cx.notify())
            .detach();
        let poll = cx.spawn(async move |weak, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                if weak.update(cx, |this, cx| this.refresh(cx)).is_err() {
                    break;
                }
                #[cfg(all(feature = "ui-qualification", target_os = "linux"))]
                cx.update(|cx| {
                    let action = weak
                        .update(cx, |this, _| {
                            let action = this
                                .probe
                                .as_mut()
                                .and_then(|probe| probe.next(&this.snapshot));
                            action
                        })
                        .ok()
                        .flatten();
                    if let Some(action) = action {
                        if let Some(handle) = cx.windows().first().copied() {
                            let _ = handle.update(cx, |_, window, cx| {
                                crate::qualification::execute(action, window, cx)
                            });
                        }
                    }
                });
            }
        });
        Self {
            native_icon: None,
            worker: Worker::spawn(connection),
            snapshot: Snapshot {
                busy: true,
                ..Snapshot::default()
            },
            search,
            names: BTreeMap::new(),
            expanded: BTreeSet::new(),
            channel: None,
            settings: false,
            tray_available: None,
            feedback: None,
            _poll: poll,
            #[cfg(all(feature = "ui-qualification", target_os = "linux"))]
            probe: None,
            #[cfg(all(feature = "status-tray", target_os = "linux"))]
            tray: beacn_linx_core::tray::Tray::spawn(),
        }
    }
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let next = self.worker.snapshot();
        let mut changed = next.catalog.as_ref().map(|c| (&c.core, &c.applications))
            != self
                .snapshot
                .catalog
                .as_ref()
                .map(|c| (&c.core, &c.applications))
            || next.selected != self.snapshot.selected
            || next.routing.assignments != self.snapshot.routing.assignments
            || next.private_session != self.snapshot.private_session
            || Status::from_snapshot(&next) != Status::from_snapshot(&self.snapshot)
            || next.connected != self.snapshot.connected
            || next.busy != self.snapshot.busy
            || next.message != self.snapshot.message
            || next.link.phase != self.snapshot.link.phase;
        if self.snapshot.catalog.as_ref().map(|c| &c.core) != next.catalog.as_ref().map(|c| &c.core)
        {
            self.names.clear();
            self.expanded.clear();
            self.channel = None;
        }
        self.snapshot = next;
        self.names.retain(|id, _| {
            self.snapshot
                .catalog
                .as_ref()
                .is_some_and(|c| c.applications.contains_key(id))
        });
        self.expanded.retain(|id| {
            self.snapshot
                .catalog
                .as_ref()
                .is_some_and(|c| c.applications.contains_key(id))
        });
        #[cfg(all(feature = "status-tray", target_os = "linux"))]
        {
            use beacn_linx_core::tray;
            let availability = self.tray.available();
            if self.tray_available != availability {
                self.tray_available = availability;
                changed = true;
            }
            self.tray.update(
                Status::from_snapshot(&self.snapshot),
                self.snapshot.link.phase == Phase::Running,
            );
            let actions = self.tray.actions();
            if actions & tray::STOP != 0 {
                self.send(Command::Stop, cx);
            }
            if actions & tray::SHOW != 0 {
                cx.activate(true);
                for window in cx.windows() {
                    let _ = window.update(cx, |_, window, _| window.activate_window());
                }
            }
            if actions & tray::QUIT != 0 {
                self.worker.stop();
                cx.quit();
            }
        }
        if changed {
            cx.notify();
        }
    }
    fn send(&mut self, command: Command, cx: &mut Context<Self>) {
        if let Command::Select(id, alias) = &command {
            if self.snapshot.selected.get(id) == Some(alias) {
                return;
            }
        }
        self.feedback = self.worker.send(command, &self.snapshot).err();
        cx.notify();
    }
    fn toggle_app(&mut self, id: ApplicationId, window: &mut Window, cx: &mut Context<Self>) {
        if !self.expanded.remove(&id) {
            if !self.names.contains_key(&id) {
                if let Some(app) = self
                    .snapshot
                    .catalog
                    .as_ref()
                    .and_then(|c| c.applications.get(&id))
                {
                    let name = self
                        .snapshot
                        .selected
                        .get(&id)
                        .cloned()
                        .unwrap_or_else(|| model::default_alias(app));
                    let input = cx.new(|cx| {
                        InputState::new(window, cx)
                            .default_value(name)
                            .placeholder("Published name")
                    });
                    cx.subscribe_in(&input, window, |_, _, _: &InputEvent, _, cx| cx.notify())
                        .detach();
                    self.names.insert(id.clone(), input);
                }
            }
            self.expanded.insert(id);
        }
        cx.notify();
    }
    fn row(
        &self,
        app: &beacn_link_protocol::catalog::CatalogApplication,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = app.id.clone();
        let selected = self.snapshot.selected.contains_key(&id);
        let expanded = self.expanded.contains(&id);
        let live = self.snapshot.connected && model::live(app);
        let select_id = id.clone();
        let details_id = id.clone();
        let save_id = id.clone();
        let alias = self
            .snapshot
            .selected
            .get(&id)
            .cloned()
            .unwrap_or_else(|| model::default_alias(app));
        let observed_name = model::app_name(app);
        let display_name = if selected {
            alias.clone()
        } else {
            observed_name.clone()
        };
        let pair = self
            .snapshot
            .routing
            .assignments
            .get(&id)
            .copied()
            .filter(|p| *p != 0);
        let status = pair.map(|p| format!("Link {p}")).unwrap_or_else(|| {
            if selected {
                "Selected".into()
            } else {
                "Available".into()
            }
        });
        let mut row = div()
            .flex()
            .flex_col()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(rgb(LINE))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .size_8()
                            .rounded_md()
                            .bg(rgb(0x252a35))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(rgb(TEAL))
                            .child(
                                model::app_name(app)
                                    .chars()
                                    .next()
                                    .unwrap_or('?')
                                    .to_uppercase()
                                    .to_string(),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .truncate()
                                    .child(display_name.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(if selected { TEAL } else { MUTED }))
                                    .child(if display_name != observed_name {
                                        format!("{observed_name} / {status}")
                                    } else {
                                        status
                                    }),
                            ),
                    )
                    .child(
                        Button::new(SharedString::from(format!("details-{}", id.0)))
                            .ghost()
                            .xsmall()
                            .icon(if expanded {
                                IconName::ChevronUp
                            } else {
                                IconName::ChevronDown
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.toggle_app(details_id.clone(), window, cx)
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!("select-{}", id.0)))
                            .small()
                            .label(if selected { "Remove" } else { "Select" })
                            .disabled(!live)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.send(
                                    if selected {
                                        Command::Unselect(select_id.clone())
                                    } else {
                                        Command::Select(select_id.clone(), alias.clone())
                                    },
                                    cx,
                                );
                            })),
                    ),
            );
        if expanded {
            let mut details = div().flex().flex_col().gap_2().pl_10().pb_1().child(
                div().text_xs().text_color(rgb(MUTED)).child(format!(
                    "Process {}  /  {} audio stream(s)",
                    app.pid,
                    app.streams.len()
                )),
            );
            if let Some(input) = self.names.get(&id) {
                let name = input.clone();
                let changed = self
                    .snapshot
                    .selected
                    .get(&id)
                    .is_none_or(|alias| input.read(cx).value().to_string() != *alias);
                details = details.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Input::new(input)
                                .id(SharedString::from(format!("name-{}", id.0)))
                                .small()
                                .disabled(!live),
                        )
                        .child(
                            Button::new(SharedString::from(format!("save-{}", id.0)))
                                .small()
                                .label(if selected {
                                    "Save name"
                                } else {
                                    "Select with name"
                                })
                                .disabled(!live || !changed)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.send(
                                        Command::Select(
                                            save_id.clone(),
                                            name.read(cx).value().to_string(),
                                        ),
                                        cx,
                                    );
                                })),
                        ),
                );
            }
            row = row.child(details);
        }
        row.into_any_element()
    }
}
impl Render for Interface {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = Status::from_snapshot(&self.snapshot);
        if self.native_icon != Some(status) {
            crate::window_icon::update(window, status);
            self.native_icon = Some(status);
        }
        let running = self.snapshot.link.phase == Phase::Running;
        #[cfg(not(all(feature = "status-tray", target_os = "linux")))]
        let tray_unavailable = false;
        #[cfg(all(feature = "status-tray", target_os = "linux"))]
        let tray_unavailable = self.tray_available == Some(false);
        let discovery = format!(
            "{}{}",
            if self.snapshot.connected {
                "Discovery online"
            } else {
                "Discovery offline"
            },
            if tray_unavailable {
                " / tray unavailable"
            } else {
                ""
            }
        );
        let start = self.snapshot.private_session
            && self.snapshot.connected
            && !self.snapshot.selected.is_empty()
            && !running
            && self.snapshot.link.phase != Phase::NeedsReview;
        let filter = self.search.read(cx).value().to_lowercase();
        let mut apps = self
            .snapshot
            .catalog
            .as_ref()
            .map(|c| c.applications.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        apps.sort_by_key(|a| {
            (
                !self.snapshot.selected.contains_key(&a.id),
                model::app_name(a).to_lowercase(),
                a.id.clone(),
            )
        });
        apps.retain(|a| {
            model::app_name(a).to_lowercase().contains(&filter)
                || self
                    .snapshot
                    .selected
                    .get(&a.id)
                    .is_some_and(|name| name.to_lowercase().contains(&filter))
                || a.binaries
                    .iter()
                    .any(|b| b.to_lowercase().contains(&filter))
        });
        let rows = apps.iter().map(|app| self.row(app, cx)).collect::<Vec<_>>();
        let narrow = f32::from(window.viewport_size().width) < 600.;
        let channel_min = if narrow {
            (f32::from(window.viewport_size().width) - 40.) / 2.
        } else {
            120.
        };
        let mut channels = div().flex().flex_wrap().gap_2();
        for pair in 1..=4u8 {
            let names = self
                .snapshot
                .routing
                .assignments
                .iter()
                .filter(|(_, p)| **p == pair)
                .filter_map(|(id, _)| self.snapshot.selected.get(id))
                .cloned()
                .collect::<Vec<_>>();
            let title = if pair == 1 {
                "Link In".to_owned()
            } else {
                format!("Link {pair} In")
            };
            channels = channels.child(
                div()
                    .id(SharedString::from(format!("channel-{pair}")))
                    .flex_1()
                    .min_w(px(channel_min))
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(if self.channel == Some(pair) {
                        TEAL
                    } else {
                        LINE
                    }))
                    .bg(rgb(PANEL))
                    .px_3()
                    .py_2()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.channel = if this.channel == Some(pair) {
                            None
                        } else {
                            Some(pair)
                        };
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(div().text_xs().text_color(rgb(MUTED)).child(title))
                            .child(
                                div()
                                    .size(px(5.))
                                    .rounded_full()
                                    .bg(rgb(if names.is_empty() { 0x4c5260 } else { TEAL })),
                            ),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_sm()
                            .truncate()
                            .child(if names.is_empty() {
                                "Unassigned".into()
                            } else {
                                names.join(", ")
                            }),
                    ),
            );
        }
        let mut body = div()
            .id("linx-window")
            .overflow_y_scroll()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .text_sm()
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(LINE))
                    .child(
                        svg()
                            .path(icon_path(status))
                            .size_8()
                            .text_color(color(status)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(div().font_weight(FontWeight::SEMIBOLD).child("LinX"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(color(status))
                                    .child(status.label()),
                            ),
                    )
                    .child(
                        Button::new("session")
                            .primary()
                            .small()
                            .label(if running { "Stop link" } else { "Start link" })
                            .disabled(!running && !start)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.send(
                                    if running {
                                        Command::Stop
                                    } else {
                                        Command::Start
                                    },
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new("settings")
                            .ghost()
                            .small()
                            .icon(IconName::Settings)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.settings = !this.settings;
                                cx.notify();
                            })),
                    ),
            )
            .child(div().flex_shrink_0().px_4().pt_3().pb_2().child(channels));
        if let Some(pair) = self.channel {
            body = body.child(
                div()
                    .flex_shrink_0()
                    .mx_4()
                    .mb_2()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(PANEL))
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child(format!(
                        "Link {pair}: assignments come from BEACN App on the streaming PC. {}",
                        if running {
                            "Showing verified private-test routes."
                        } else {
                            "No active routing session."
                        }
                    )),
            );
        }
        if self.settings {
            body=body.child(div().mx_4().mb_2().p_3().rounded_lg().border_1().border_color(rgb(LINE)).bg(rgb(PANEL)).flex().flex_col().gap_1()
                .child(div().font_weight(FontWeight::MEDIUM).child("About this preview"))
                .child(div().text_xs().text_color(rgb(MUTED)).child("LinX 0.1.0 / Independent Linux companion for BEACN Studio"))
                .child(div().text_xs().text_color(rgb(MUTED)).child("Selections are kept until you close the app. Device connection and desktop routing are still disabled.")));
        }
        body = body
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .px_4()
                    .py_2()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .font_weight(FontWeight::MEDIUM)
                            .child("Applications"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(format!("{} selected", self.snapshot.selected.len())),
                    ),
            )
            .child(
                div().px_4().pb_2().child(
                    Input::new(&self.search)
                        .id("search-input")
                        .small()
                        .cleanable(true)
                        .prefix(gpui_kit::component::Icon::new(IconName::Search).small()),
                ),
            );
        if let Some(message) = self.feedback.as_ref().or(self.snapshot.message.as_ref()) {
            body = body.child(
                div()
                    .flex_shrink_0()
                    .px_4()
                    .pb_2()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child(model::label(message)),
            );
        }
        let empty = rows.is_empty();
        body = body
            .child(
                div()
                    .id("application-list")
                    .flex_1()
                    .min_h(px(120.))
                    .overflow_y_scroll()
                    .mx_4()
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(LINE))
                    .bg(rgb(PANEL))
                    .children(rows)
                    .when(empty, |el| {
                        el.child(
                            div()
                                .p_5()
                                .text_color(rgb(MUTED))
                                .child(if self.snapshot.busy {
                                    "Looking for audio applications..."
                                } else if !self.snapshot.connected {
                                    "Application discovery is unavailable."
                                } else if !filter.is_empty() {
                                    "No matching applications."
                                } else {
                                    "Play audio in an app to see it here."
                                }),
                        )
                    }),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .px_4()
                    .py_2()
                    .flex()
                    .justify_between()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child(discovery)
                    .child(if self.snapshot.private_session {
                        "Private test / no USB"
                    } else {
                        "Preview / routing off"
                    }),
            );
        body
    }
}
pub fn run(connection: Connection) {
    launch(
        connection,
        #[cfg(all(feature = "ui-qualification", target_os = "linux"))]
        None,
    );
}
#[cfg(all(feature = "ui-qualification", target_os = "linux"))]
pub fn run_qualification(connection: Connection, probe: crate::qualification::Configuration) {
    launch(connection, Some(probe));
}
fn launch(
    connection: Connection,
    #[cfg(all(feature = "ui-qualification", target_os = "linux"))] probe: Option<
        crate::qualification::Configuration,
    >,
) {
    application().with_assets(Assets).run(move |cx| {
        init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(720.), px(520.)),
                    cx,
                ))),
                window_min_size: Some(size(px(420.), px(380.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("LinX".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let view = Interface::new(connection, window, cx);
                    #[cfg(all(feature = "ui-qualification", target_os = "linux"))]
                    let view = {
                        let mut view = view;
                        view.probe = probe.map(crate::qualification::Probe::new);
                        view
                    };
                    view
                })
            },
        )
        .expect("open LinX window");
        cx.activate(true);
    });
}
