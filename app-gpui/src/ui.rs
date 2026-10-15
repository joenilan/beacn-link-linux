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
    catalog_open: bool,
    appearance: crate::appearance::Appearance,
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
            catalog_open: true,
            appearance: Default::default(),
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
            || next.session_available != self.snapshot.session_available
            || Status::from_snapshot(&next) != Status::from_snapshot(&self.snapshot)
            || next.connected != self.snapshot.connected
            || next.busy != self.snapshot.busy
            || next.message != self.snapshot.message
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
impl Interface {
    fn console_strip(&self, pair: u8, width: f32, cx: &mut Context<Self>) -> AnyElement {
        let p = self.appearance.colors();
        let fader_height = if self.catalog_open || self.settings {
            64.
        } else {
            142.
        };
        let accent = p.channels[(pair - 1) as usize];
        let names = self
            .snapshot
            .routing
            .assignments
            .iter()
            .filter(|(_, p)| **p == pair)
            .filter_map(|(id, _)| self.snapshot.selected.get(id))
            .cloned()
            .collect::<Vec<_>>();
        let mut well = div()
            .flex()
            .flex_col()
            .gap_1()
            .min_h(px(36.))
            .bg(rgb(p.well))
            .rounded_sm()
            .p_2();
        if names.is_empty() {
            well = well.child(
                div()
                    .text_xs()
                    .text_color(rgb(p.muted))
                    .child("Not assigned"),
            );
        }
        for name in names {
            well = well.child(div().text_xs().truncate().child(name));
        }
        let title = if pair == 1 {
            "Link In".to_string()
        } else {
            format!("Link {pair} In")
        };
        div()
            .id(SharedString::from(format!("channel-{pair}")))
            .w(px(width))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(rgb(p.surface))
            .border_t_2()
            .border_color(rgb(accent))
            .px_3()
            .py_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(title))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(p.muted))
                            .child(format!("{pair:02}")),
                    ),
            )
            .child(
                div()
                    .mt_1()
                    .text_xs()
                    .text_color(rgb(p.muted))
                    .child("Linux send"),
            )
            .child(
                div()
                    .mt_4()
                    .mb_3()
                    .h(px(fader_height))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .child(
                        div()
                            .h_full()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .text_color(rgb(p.muted))
                            .text_size(px(9.))
                            .children(
                                ["0", "−6", "−12", "−24", "−48"]
                                    .into_iter()
                                    .map(|label| div().child(label)),
                            ),
                    )
                    .child(
                        div()
                            .relative()
                            .w(px(28.))
                            .h_full()
                            .opacity(0.45)
                            .child(
                                div()
                                    .absolute()
                                    .left(px(12.))
                                    .top_0()
                                    .h_full()
                                    .w(px(4.))
                                    .rounded_sm()
                                    .bg(rgb(p.track)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left_0()
                                    .top(px(fader_height / 2. - 6.5))
                                    .w(px(28.))
                                    .h(px(13.))
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(rgb(p.line))
                                    .bg(rgb(p.cap))
                                    .child(div().mt(px(5.)).h(px(2.)).w_full().bg(rgb(p.track))),
                            ),
                    )
                    .child(
                        div()
                            .h_full()
                            .flex()
                            .gap(px(3.))
                            .opacity(0.45)
                            .child(div().h_full().w(px(4.)).bg(rgb(p.track)))
                            .child(div().h_full().w(px(4.)).bg(rgb(p.track))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_1()
                    .mb_2()
                    .child(div().text_lg().text_color(rgb(p.muted)).child("—"))
                    .child(div().text_size(px(9.)).text_color(rgb(p.muted)).child("dB")),
            )
            .child(
                Button::new(SharedString::from(format!("send-mute-{pair}")))
                    .small()
                    .label("Mute")
                    .disabled(true),
            )
            .child(
                div()
                    .mt_3()
                    .mb_2()
                    .text_size(px(9.))
                    .text_color(rgb(p.muted))
                    .child("APPLICATIONS"),
            )
            .child(well)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.channel = Some(pair);
                    cx.notify();
                }),
            )
            .into_any_element()
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
        let viewport = f32::from(window.viewport_size().width);
        let columns = if viewport < 600. { 2. } else { 4. };
        let width = ((viewport - 28.) / columns - 1.).max(130.);
        let mut bank = div()
            .id("console-bank")
            .mx_3()
            .flex()
            .flex_wrap()
            .gap(px(1.));
        for pair in 1..=4 {
            bank = bank.child(self.console_strip(pair, width, cx));
        }
        let status_label = if running || status == Status::Error {
            status.label()
        } else {
            self.snapshot.startup.label()
        };
        let mut body = div()
            .id("linx-window")
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .bg(rgb(p.background))
            .text_color(rgb(p.text))
            .text_sm()
            .child(
                div()
                    .flex_shrink_0()
                    .h(px(57.))
                    .px_5()
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(rgb(p.line))
                    .child(
                        div()
                            .size(px(31.))
                            .rounded_md()
                            .bg(rgb(if self.appearance.light {
                                0x252b31
                            } else {
                                p.background
                            }))
                            .p(px(4.))
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
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("LinX"),
                    )
                    .child(
                        div()
                            .ml_2()
                            .pl_3()
                            .border_l_1()
                            .border_color(rgb(p.line))
                            .text_size(px(9.))
                            .text_color(rgb(p.muted))
                            .child("CONSOLE"),
                    )
                    .child(div().flex_1())
                    .child(div().size(px(5.)).rounded_full().bg(color(status)))
                    .child(div().text_xs().text_color(rgb(p.muted)).child(status_label))
                    .child(
                        Button::new("session")
                            .ghost()
                            .small()
                            .label(if running { "Pause" } else { "Connect" })
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
            .child(
                div()
                    .flex_shrink_0()
                    .px_5()
                    .py_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(9.))
                    .text_color(rgb(p.muted))
                    .child("STUDIO LINK")
                    .child("Assignments from streaming PC"),
            )
            .child(bank)
            .child(
                div()
                    .flex_shrink_0()
                    .px_5()
                    .py_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .text_size(px(9.))
                            .text_color(rgb(p.muted))
                            .child("LINUX → STUDIO LINK"),
                    )
                    .child(
                        div()
                            .text_size(px(9.))
                            .text_color(rgb(p.muted))
                            .child("Gain and meters unavailable"),
                    ),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .px_4()
                    .pb_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        Switch::new("auto-connect")
                            .small()
                            .label("Connect automatically")
                            .checked(self.snapshot.auto_connect)
                            .on_change(cx.listener(|this, enabled, _, cx| {
                                this.send(Command::AutoConnect(*enabled), cx)
                            })),
                    )
                    .child(
                        Button::new("applications")
                            .ghost()
                            .small()
                            .label(format!("Applications · {}", self.snapshot.selected.len()))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.catalog_open = !this.catalog_open;
                                cx.notify();
                            })),
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
            .feedback
            .as_ref()
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
                    .text_color(rgb(p.muted))
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
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(p.muted))
                                    .child("APPLICATIONS"),
                            )
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
                            }),
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
