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
        switch::Switch,
    },
    *,
};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};
#[cfg(all(feature = "native-control", target_os = "linux"))]
#[path = "control_ui.rs"]
mod control_ui;

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
fn tracked(text: &'static str, size: f32, spacing: f32) -> Div {
    div()
        .flex()
        .items_center()
        .text_size(px(size))
        .children(text.chars().enumerate().map(move |(index, ch)| {
            div()
                .when(index != 0, |e| e.ml(px(spacing)))
                .child(ch.to_string())
        }))
}
impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        if let Some(svg) = crate::appearance::tool_svg(path) {
            return Ok(Some(Cow::Owned(svg.into_bytes())));
        }
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
    catalog_open: bool,
    studio_app: bool,
    expanded_window: Option<(Size<Pixels>, Size<Pixels>)>,
    appearance: crate::appearance::Appearance,
    tray_available: Option<bool>,
    feedback: Option<String>,
    _poll: Task<()>,
    #[cfg(all(feature = "native-control", target_os = "linux"))]
    control: crate::control::Control,
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
        #[cfg(all(feature = "native-control", target_os = "linux"))]
        let controlled_window = window.window_handle();
        let poll = cx.spawn(async move |weak, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                if weak.update(cx, |this, cx| this.refresh(cx)).is_err() {
                    break;
                }
                if crate::shutdown::requested() {
                    cx.update(|cx| {
                        let _ = weak.update(cx, |this, _| this.worker.stop());
                        cx.quit();
                    });
                    break;
                }
                #[cfg(all(feature = "native-control", target_os = "linux"))]
                cx.update(|cx| {
                    let request = weak
                        .update(cx, |this, _| this.control.take())
                        .ok()
                        .flatten();
                    if let Some(request) = request {
                        let _ = controlled_window.update(cx, |_, window, cx| {
                            control_ui::dispatch(&weak, request, window, cx)
                        });
                    }
                });
                #[cfg(all(feature = "ui-qualification", target_os = "linux"))]
                cx.update(|cx| {
                    for handle in cx.windows() {
                        let _ = handle.update(cx, |_, window, cx| {
                            crate::diagnostics::capture(window, cx);
                        });
                    }
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
        let studio_app = match &connection {
            #[cfg(all(
                any(feature = "studio", feature = "desktop-session-test"),
                target_os = "linux"
            ))]
            Connection::Studio | Connection::DesktopStudioSession(_) => true,
            _ => false,
        };
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
            catalog_open: false,
            studio_app,
            expanded_window: None,
            appearance: Default::default(),
            tray_available: None,
            feedback: None,
            _poll: poll,
            #[cfg(all(feature = "native-control", target_os = "linux"))]
            control: crate::control::Control::from_configuration()
                .expect("configured native control endpoint"),
            #[cfg(all(feature = "ui-qualification", target_os = "linux"))]
            probe: None,
            #[cfg(all(feature = "status-tray", target_os = "linux"))]
            tray: beacn_linx_core::tray::Tray::spawn(),
        }
    }
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let next = self.worker.snapshot();
        #[cfg(all(feature = "ui-qualification", target_os = "linux"))]
        crate::diagnostics::state(&next);
        let mut changed = next.catalog.as_ref().map(|c| (&c.core, &c.applications))
            != self
                .snapshot
                .catalog
                .as_ref()
                .map(|c| (&c.core, &c.applications))
            || next.selected != self.snapshot.selected
            || next.routing.assignments != self.snapshot.routing.assignments
            || next.private_session != self.snapshot.private_session
            || next.session_available != self.snapshot.session_available
            || Status::from_snapshot(&next) != Status::from_snapshot(&self.snapshot)
            || next.connected != self.snapshot.connected
            || next.busy != self.snapshot.busy
            || next.message != self.snapshot.message
            || next.link.error != self.snapshot.link.error
            || next.link.phase != self.snapshot.link.phase;
        changed |= next.auto_connect != self.snapshot.auto_connect
            || next.startup != self.snapshot.startup
            || next.preference_error != self.snapshot.preference_error;
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
    fn fit_drawers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.catalog_open || self.settings {
            let current = window.viewport_size();
            if self.expanded_window.is_none() && current.height <= px(650.) {
                let available = window
                    .display(cx)
                    .map(|d| d.bounds().size.height - px(80.))
                    .unwrap_or(px(820.));
                let desired = if self.catalog_open {
                    px(820.)
                } else {
                    px(740.)
                };
                let expanded = size(current.width, desired.min(available).max(current.height));
                if expanded.height > current.height {
                    self.expanded_window = Some((current, expanded));
                    window.resize(expanded);
                }
            }
        } else if let Some((original, expanded)) = self.expanded_window.take() {
            if window.viewport_size() == expanded {
                window.resize(original);
            }
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
        let palette = self.appearance.colors();
        let id = app.id.clone();
        let selected = self.snapshot.selected.contains_key(&id);
        let expanded = self.expanded.contains(&id);
        let live = self.snapshot.connected && model::live(app);
        let select_id = id.clone();
        let details_id = id.clone();
        let save_id = id.clone();
        let route_id = id.clone();
        let target_pair = self.channel.unwrap_or(1);
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
                if self.snapshot.link.error.is_some() {
                    "Selected · session stopped".into()
                } else if self.snapshot.link.phase == Phase::Running {
                    "Selected · waiting for assignment".into()
                } else {
                    "Selected · ready to publish".into()
                }
            } else {
                "Available".into()
            }
        });
        let route_alias = alias.clone();
        let mut row = div()
            .flex()
            .flex_col()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(rgb(palette.line))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .size_8()
                            .rounded_md()
                            .bg(rgb(palette.well))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(rgb(palette.channels[0]))
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
                                    .text_color(rgb(if selected {
                                        palette.channels[0]
                                    } else {
                                        palette.muted
                                    }))
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
                            .disabled(
                                !live
                                    || (self.studio_app
                                        && !selected
                                        && (!self.snapshot.selected.is_empty()
                                            || app.streams.len() != 1)),
                            )
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
        if self.studio_app {
            row = row.child(
                Button::new(SharedString::from(format!("route-{target_pair}-{}", id.0)))
                    .small()
                    .label(if target_pair == 1 {
                        "Assign to Link In".to_owned()
                    } else {
                        format!("Assign to Link {target_pair} In")
                    })
                    .disabled(
                        !live
                            || self.snapshot.link.phase == Phase::NeedsReview
                            || (!selected
                                && (!self.snapshot.selected.is_empty() || app.streams.len() != 1)),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.send(
                            Command::Assign(route_id.clone(), route_alias.clone(), target_pair),
                            cx,
                        )
                    })),
            );
        }
        if expanded {
            let mut details = div().flex().flex_col().gap_2().pl_10().pb_1().child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.muted))
                    .child(format!(
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
        let p = self.appearance.colors();
        let status = Status::from_snapshot(&self.snapshot);
        if self.native_icon != Some(status) {
            crate::window_icon::update(window, status);
            self.native_icon = Some(status);
        }
        let running = self.snapshot.link.phase == Phase::Running;
        let start = self.snapshot.session_available
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
                    .is_some_and(|n| n.to_lowercase().contains(&filter))
                || a.binaries
                    .iter()
                    .any(|b| b.to_lowercase().contains(&filter))
        });
        let narrow = f32::from(window.viewport_size().width) < 700.;
        let mut bank = div()
            .id("console-bank")
            .mx(px(13.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(px(1.));
        let count = if narrow { 2 } else { 4 };
        for first in (1..=4u8).step_by(count) {
            let mut row = div().w_full().flex().flex_nowrap();
            for pair in first..first + count as u8 {
                let names = self
                    .snapshot
                    .routing
                    .assignments
                    .iter()
                    .filter(|(_, assigned)| **assigned == pair)
                    .filter_map(|(id, _)| self.snapshot.selected.get(id))
                    .cloned()
                    .collect();
                row = row.child(crate::console::strip(
                    pair,
                    names,
                    if self.snapshot.selected.is_empty()
                        || (running
                            && self.snapshot.selected.keys().all(|id| {
                                self.snapshot
                                    .routing
                                    .assignments
                                    .get(id)
                                    .is_some_and(|pair| *pair != 0)
                            }))
                    {
                        "Choose application"
                    } else if self.snapshot.link.error.is_some() {
                        "Session stopped"
                    } else if running {
                        "Waiting for assignment"
                    } else {
                        "Connect to publish"
                    },
                    self.appearance.light,
                    p.channels[(pair - 1) as usize],
                    cx.listener(move |this, _, window, cx| {
                        let same = this.catalog_open && this.channel == Some(pair);
                        this.channel = Some(pair);
                        this.catalog_open = !same;
                        this.fit_drawers(window, cx);
                        cx.notify();
                    }),
                ));
            }
            #[cfg(all(feature = "ui-qualification", target_os = "linux"))]
            {
                row = row.on_children_prepainted(|bounds, window, _| {
                    crate::diagnostics::row(&bounds, window)
                });
            }
            bank = bank.child(row);
        }
        let status_label = if running || status == Status::Error {
            status.label()
        } else {
            self.snapshot.startup.label()
        };
        let tool_color = if self.appearance.light {
            0x43515b
        } else {
            0xcdd1d7
        };
        let mut body = div()
            .id("linx-window")
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .bg(rgb(p.background))
            .text_color(rgb(p.text))
            .text_size(px(14.))
            .font_family("Segoe UI")
            .border_1()
            .border_color(rgb(if self.appearance.light {
                0xb9c2c2
            } else {
                0x3b3d41
            }))
            .rounded(px(10.))
            .child(
                div()
                    .id("console-header")
                    .flex_shrink_0()
                    .h(px(57.))
                    .px(px(20.))
                    .flex()
                    .items_center()
                    .gap(px(9.))
                    .border_b_1()
                    .border_color(rgb(if self.appearance.light {
                        0xdce1dc
                    } else {
                        0x2d2f32
                    }))
                    .child(
                        div()
                            .size(px(if self.appearance.light { 31. } else { 23. }))
                            .flex_shrink_0()
                            .rounded(px(7.))
                            .when(self.appearance.light, |e| e.bg(rgb(0x252b31)).p(px(5.)))
                            .child(
                                svg()
                                    .path(icon_path(status))
                                    .size_full()
                                    .text_color(color(status)),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(19.))
                            .line_height(px(25.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .flex()
                            .children("LinX".chars().enumerate().map(|(index, ch)| {
                                div()
                                    .when(index != 0, |e| e.ml(px(-1.14)))
                                    .child(ch.to_string())
                            })),
                    )
                    .child(
                        div()
                            .ml(px(4.))
                            .pl(px(10.))
                            .border_l_1()
                            .border_color(rgb(p.line))
                            .text_size(px(9.))
                            .line_height(px(13.))
                            .text_color(rgb(p.muted))
                            .child(tracked("STUDIO", 9., 0.9)),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .mr(px(7.))
                            .child(div().size(px(5.)).rounded_full().bg(color(status)))
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(p.muted))
                                    .child(status_label),
                            ),
                    )
                    .child(
                        Button::new("session")
                            .ghost()
                            .small()
                            .h(px(29.))
                            .px(px(9.))
                            .border_1()
                            .border_color(rgb(p.line))
                            .rounded(px(5.))
                            .text_color(rgb(tool_color))
                            .disabled(!running && !start)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        svg()
                                            .path(if running {
                                                "console/pause.svg"
                                            } else {
                                                "console/play.svg"
                                            })
                                            .size(px(13.))
                                            .text_color(rgb(tool_color)),
                                    )
                                    .child(div().text_size(px(12.)).child(if running {
                                        "Pause"
                                    } else {
                                        "Connect"
                                    })),
                            )
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
                            .ml(px(7.))
                            .size(px(29.))
                            .p_0()
                            .border_1()
                            .border_color(rgb(p.line))
                            .rounded(px(5.))
                            .text_color(rgb(tool_color))
                            .child(
                                svg()
                                    .path("console/sliders.svg")
                                    .size(px(13.))
                                    .text_color(rgb(tool_color)),
                            )
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.settings = !this.settings;
                                this.fit_drawers(window, cx);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .id("console-subheader")
                    .flex_shrink_0()
                    .h(px(46.))
                    .px(px(23.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(9.))
                            .text_color(rgb(p.channels[0]))
                            .child(tracked("LINUX SENDS", 9., 1.17)),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(p.muted))
                            .child("4 stereo channels"),
                    ),
            )
            .child(bank)
            .child(
                div()
                    .id("console-footer")
                    .flex_shrink_0()
                    .h(px(42.))
                    .px(px(21.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .text_size(px(9.))
                            .text_color(rgb(p.muted))
                            .child("THIS PC")
                            .child(
                                svg()
                                    .path("console/arrow.svg")
                                    .size(px(12.))
                                    .text_color(rgb(p.muted)),
                            )
                            .child("STUDIO")
                            .child(
                                svg()
                                    .path("console/arrow.svg")
                                    .size(px(12.))
                                    .text_color(rgb(p.muted)),
                            )
                            .child("STREAMING PC"),
                    )
                    .child(
                        Button::new("mini-view")
                            .ghost()
                            .small()
                            .h(px(14.))
                            .p_0()
                            .disabled(true)
                            .tooltip("Mini view is not implemented yet")
                            .text_color(rgb(p.channels[0]))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(
                                        svg()
                                            .path("console/expand.svg")
                                            .size(px(12.))
                                            .text_color(rgb(p.channels[0])),
                                    )
                                    .child(div().text_size(px(10.)).child("Mini view")),
                            ),
                    ),
            );
        if self.settings {
            let mut accents = div().flex().gap_2();
            for accent in crate::appearance::Accent::ALL {
                accents = accents.child(
                    Button::new(SharedString::from(format!(
                        "accent-{}",
                        accent.label().to_lowercase()
                    )))
                    .ghost()
                    .small()
                    .label(accent.label())
                    .bg(rgb(if self.appearance.accent == accent {
                        p.surface
                    } else {
                        p.background
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.appearance.accent = accent;
                        cx.notify();
                    })),
                );
            }
            body = body.child(
                div()
                    .mx_4()
                    .mb_3()
                    .p_3()
                    .border_1()
                    .border_color(rgb(p.line))
                    .rounded_md()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        Switch::new("auto-connect")
                            .small()
                            .label("Connect automatically while LinX is open")
                            .checked(self.snapshot.auto_connect)
                            .on_change(cx.listener(|this, enabled, _, cx| {
                                this.send(Command::AutoConnect(*enabled), cx)
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(p.muted))
                            .child("Gain, mute and meters are not connected yet."),
                    )
                    .child(
                        Button::new("applications-settings")
                            .ghost()
                            .small()
                            .label("Applications")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.catalog_open = !this.catalog_open;
                                this.fit_drawers(window, cx);
                                cx.notify();
                            })),
                    )
                    .child(div().text_xs().text_color(rgb(p.muted)).child("Appearance"))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("theme-dark")
                                    .ghost()
                                    .small()
                                    .label("Dark")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.appearance.light = false;
                                        Theme::change(ThemeMode::Dark, Some(window), cx);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("theme-light")
                                    .ghost()
                                    .small()
                                    .label("Light")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.appearance.light = true;
                                        Theme::change(ThemeMode::Light, Some(window), cx);
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(accents),
            );
        }
        if let Some(message) = self
            .snapshot
            .link
            .error
            .as_ref()
            .or(self.feedback.as_ref())
            .or(self.snapshot.preference_error.as_ref())
            .or(self
                .snapshot
                .message
                .as_ref()
                .filter(|_| status == Status::Error))
        {
            body = body.child(
                div()
                    .flex_shrink_0()
                    .px_4()
                    .pb_2()
                    .text_xs()
                    .text_color(if self.snapshot.link.error.is_some() {
                        color(Status::Error)
                    } else {
                        rgb(p.muted)
                    })
                    .child(model::label(message)),
            );
        }
        if self.catalog_open {
            let empty = apps.is_empty();
            let rows = apps.iter().map(|app| self.row(app, cx)).collect::<Vec<_>>();
            body = body.child(
                div()
                    .mx_3()
                    .mb_3()
                    .border_t_1()
                    .border_color(rgb(p.line))
                    .pt_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .px_1()
                            .child(div().text_xs().text_color(rgb(p.muted)).child(
                                if self.studio_app {
                                    let pair = self.channel.unwrap_or(1);
                                    if pair == 1 {
                                        "APPLICATIONS → LINK IN".to_owned()
                                    } else {
                                        format!("APPLICATIONS → LINK {pair} IN")
                                    }
                                } else {
                                    "APPLICATIONS".to_owned()
                                },
                            ))
                            .child(
                                div().flex_1().child(
                                    Input::new(&self.search)
                                        .id("search-input")
                                        .small()
                                        .cleanable(true)
                                        .prefix(
                                            gpui_kit::component::Icon::new(IconName::Search)
                                                .small(),
                                        ),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .id("application-list")
                            .max_h(px(210.))
                            .min_h(px(52.))
                            .overflow_y_scroll()
                            .bg(rgb(p.well))
                            .rounded_sm()
                            .children(rows)
                            .when(empty, |el| {
                                el.child(div().p_3().text_xs().text_color(rgb(p.muted)).child(
                                    if self.snapshot.busy {
                                        "Looking for audio applications…"
                                    } else if !self.snapshot.connected {
                                        "Application discovery is unavailable."
                                    } else if !filter.is_empty() {
                                        "No matching applications."
                                    } else {
                                        "Play audio in an app to see it here."
                                    },
                                ))
                            })
                            .test_support(),
                    ),
            );
        }
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
    crate::shutdown::install();
    application().with_assets(Assets).run(move |cx| {
        #[cfg(all(feature = "studio-session-test", target_os = "linux"))]
        if probe.as_ref().is_some_and(|probe| probe.studio) {
            eprintln!("STUDIO GUI runtime initialized");
        }
        init(cx);
        #[cfg(all(feature = "studio-session-test", target_os = "linux"))]
        if probe.as_ref().is_some_and(|probe| probe.studio) {
            eprintln!("STUDIO GUI components initialized");
        }
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
                    size(px(788.), px(568.)),
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
                    #[cfg(all(feature = "studio-session-test", target_os = "linux"))]
                    if probe.as_ref().is_some_and(|probe| probe.studio) {
                        eprintln!("STUDIO GUI window opened");
                    }
                    let view = Interface::new(connection, window, cx);
                    #[cfg(all(feature = "ui-qualification", target_os = "linux"))]
                    let view = {
                        let mut view = view;
                        if probe.is_some() {
                            view.catalog_open = true;
                        }
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
