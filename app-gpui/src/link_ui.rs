//! LinX Console for the persistent Link In channels. Each channel strip shows
//! the applications currently routed to it (read back from PipeWire) and its
//! live levels. Choosing a channel opens the application list, where any
//! application can be sent to Link In 1–4 or back to the desktop output.
use beacn_linx_core::{
    icon::Status,
    link::{LinkApp, LinkCommand, LinkView, LinkWorker, Phase},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{
        IconName, Sizable, Theme, ThemeMode,
        button::{Button, ButtonVariants},
        input::{Input, InputEvent, InputState},
        switch::Switch,
    },
    *,
};
use std::time::{Duration, Instant};

#[cfg(all(feature = "native-control", target_os = "linux"))]
#[path = "link_control.rs"]
mod link_control;

/// Control-safe identifier for an application key.
pub(crate) fn key_id(key: &str) -> String {
    key.bytes().map(|b| format!("{b:02x}")).collect()
}

fn status_of(view: &LinkView) -> Status {
    match view.phase {
        Phase::Starting => Status::Connecting,
        Phase::Paused => Status::Disconnected,
        Phase::Retrying => Status::Error,
        Phase::Running if view.studio && view.audio_ready => Status::Connected,
        Phase::Running => Status::WaitingAudio,
    }
}

/// App-wide owner of the Link worker and tray icon. LinX keeps routing while
/// its window is closed; the tray reopens it.
pub(crate) struct LinkCore {
    pub(crate) worker: LinkWorker,
    /// Arguments used for launch at login.
    login_args: Vec<String>,
    /// Started hidden and no window shown yet.
    hidden_since: Option<std::time::Instant>,
    autostart: bool,
    #[cfg(all(feature = "status-tray", target_os = "linux"))]
    tray: beacn_linx_core::tray::Tray,
}
impl Global for LinkCore {}
impl LinkCore {
    pub(crate) fn stop(cx: &mut App) {
        if cx.has_global::<LinkCore>() {
            cx.global_mut::<LinkCore>().worker.stop();
        }
    }
}

#[derive(Default)]
struct ChannelMeter {
    slot: Option<usize>,
    freshness: beacn_link_protocol::metering::Freshness,
    ballistics: crate::meter::Ballistics,
    display: Option<crate::meter::Display>,
}

pub struct LinkInterface {
    pub(crate) view: LinkView,
    native_icon: Option<Status>,
    pub(crate) search: Entity<InputState>,
    pub(crate) channel: u8,
    pub(crate) settings: bool,
    pub(crate) catalog_open: bool,
    pub(crate) appearance: crate::appearance::Appearance,
    pub(crate) feedback: Option<String>,
    meters: [ChannelMeter; 4],
    /// Level just set from the UI, shown until PipeWire echoes it.
    pending_levels: [Option<(crate::console::Level, Instant)>; 4],
    _poll: Task<()>,
    #[cfg(all(feature = "native-control", target_os = "linux"))]
    control: crate::control::Control,
}

impl LinkInterface {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Closing never stops routing: to the tray when there is one,
        // otherwise minimized to the dock. Quit is explicit.
        window.on_window_should_close(cx, |window, cx| close_requested(window, cx));
        // No invisible shadow margin around the self-drawn window: the
        // console uses the whole window, resize edges sit inside it.
        window.set_client_inset(px(0.));
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Find an application"));
        cx.subscribe_in(&search, window, |_, _, _: &InputEvent, _, cx| cx.notify())
            .detach();
        #[cfg(all(feature = "native-control", target_os = "linux"))]
        let controlled_window = window.window_handle();
        let poll = cx.spawn(async move |weak, cx| {
            loop {
                // Fast polling only while a meter can move.
                let active = weak
                    .update(cx, |this, _| {
                        this.view.phase == Phase::Running
                            && this.view.apps.iter().any(|a| a.running && a.channel > 0)
                    })
                    .unwrap_or(false);
                cx.background_executor()
                    .timer(Duration::from_millis(if active { 33 } else { 100 }))
                    .await;
                if weak.update(cx, |this, cx| this.refresh(cx)).is_err() {
                    break;
                }
                #[cfg(all(feature = "ui-qualification", target_os = "linux"))]
                cx.update(|cx| {
                    for handle in cx.windows() {
                        let _ = handle.update(cx, |_, window, cx| {
                            crate::diagnostics::capture(window, cx);
                        });
                    }
                });
                #[cfg(all(feature = "native-control", target_os = "linux"))]
                cx.update(|cx| {
                    let request = weak
                        .update(cx, |this, _| this.control.take())
                        .ok()
                        .flatten();
                    if let Some(request) = request {
                        let _ = controlled_window.update(cx, |_, window, cx| {
                            link_control::dispatch(&weak, request, window, cx)
                        });
                    }
                });
            }
        });
        Self {
            view: cx.global::<LinkCore>().worker.view(),
            native_icon: None,
            search,
            channel: 1,
            settings: false,
            catalog_open: false,
            appearance: Default::default(),
            feedback: None,
            meters: Default::default(),
            pending_levels: [None; 4],
            _poll: poll,
            #[cfg(all(feature = "native-control", target_os = "linux"))]
            control: crate::control::Control::from_configuration()
                .expect("configured native control endpoint"),
        }
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let next = cx.global::<LinkCore>().worker.view();
        let mut changed = next.phase != self.view.phase
            || next.apps != self.view.apps
            || next.studio != self.view.studio
            || next.audio_ready != self.view.audio_ready
            || next.control != self.view.control
            || next.message != self.view.message
            || next.audio_error != self.view.audio_error
            || next.auto_connect != self.view.auto_connect
            || next.levels != self.view.levels;
        for pending in &mut self.pending_levels {
            if pending.is_some_and(|(_, at)| at.elapsed() > Duration::from_millis(1500)) {
                *pending = None;
                changed = true;
            }
        }
        let now = Instant::now();
        for (index, meter) in self.meters.iter_mut().enumerate() {
            let slot = next.meters[index].as_ref();
            let key = slot.map(|s| std::sync::Arc::as_ptr(s) as usize);
            if key != meter.slot {
                *meter = ChannelMeter {
                    slot: key,
                    ..Default::default()
                };
            }
            // Only meter channels that carry an application.
            let used = next.apps.iter().any(|a| a.channel as usize == index + 1);
            let reading = slot.filter(|_| used).and_then(|s| s.read());
            let sample = meter.freshness.accept(reading, now);
            let display = meter
                .ballistics
                .update(sample.map(|r| (r.peak, r.rms)), now);
            changed |= display != meter.display;
            meter.display = display;
        }
        self.view = next;
        if changed {
            cx.notify();
        }
    }

    pub(crate) fn send(&mut self, command: LinkCommand, cx: &mut Context<Self>) {
        self.feedback = cx.global::<LinkCore>().worker.send(command).err();
        cx.notify();
    }

    /// Current level for a channel's fader, preferring a just-set value.
    fn level(&self, channel: u8) -> Option<crate::console::Level> {
        let index = channel as usize - 1;
        if let Some((level, _)) = self.pending_levels[index] {
            return Some(level);
        }
        let reported = self.view.levels[index];
        (self.view.phase == Phase::Running && reported.known)
            .then(|| crate::console::Level::from_linear(reported.volume, reported.mute))
    }

    fn set_level(&mut self, channel: u8, volume: Option<f32>, mute: Option<bool>, cx: &mut Context<Self>) {
        let Some(mut level) = self.level(channel) else {
            return;
        };
        if let Some(volume) = volume {
            level.db = crate::console::Level::from_linear(volume, level.muted).db;
        }
        if let Some(mute) = mute {
            level.muted = mute;
        }
        self.pending_levels[channel as usize - 1] = Some((level, Instant::now()));
        // Faders send many updates; a full queue just drops this one.
        let _ = cx
            .global::<LinkCore>()
            .worker
            .send(LinkCommand::Level { channel, volume, mute });
        cx.notify();
    }

    pub(crate) fn filtered_apps(&self, cx: &App) -> Vec<LinkApp> {
        let filter = self.search.read(cx).value().to_lowercase();
        let mut apps: Vec<LinkApp> = self
            .view
            .apps
            .iter()
            .filter(|a| filter.is_empty() || a.name.to_lowercase().contains(&filter))
            .cloned()
            .collect();
        // Apps on the chosen channel first, then playing apps, then by name.
        apps.sort_by_key(|a| (a.channel != self.channel, !a.running, a.name.to_lowercase()));
        apps
    }

    fn row(&self, app: &LinkApp, cx: &mut Context<Self>) -> AnyElement {
        let p = self.appearance.colors();
        let id = key_id(&app.key);
        let status = match (app.channel, app.running) {
            (0, true) => "Playing · desktop output".to_owned(),
            (0, false) => "Desktop output".to_owned(),
            (n, true) => format!("Link {n} · playing"),
            (n, false) => format!("Link {n}"),
        };
        let mut chips = div().flex().items_center().gap(px(4.));
        for channel in 1..=4u8 {
            let key = app.key.clone();
            let current = app.channel == channel;
            chips = chips.child(
                Button::new(SharedString::from(format!("route-{channel}-{id}")))
                    .ghost()
                    .small()
                    .h(px(24.))
                    .w(px(26.))
                    .p_0()
                    .border_1()
                    .border_color(rgb(if current { p.channels[channel as usize - 1] } else { p.line }))
                    .bg(rgb(if current { p.surface } else { p.well }))
                    .text_color(rgb(if current { p.channels[channel as usize - 1] } else { p.muted }))
                    .tooltip(format!("Send to Link In {channel}"))
                    .child(div().text_size(px(11.)).child(channel.to_string()))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.send(LinkCommand::Assign(key.clone(), channel), cx)
                    })),
            );
        }
        let key = app.key.clone();
        chips = chips.child(
            Button::new(SharedString::from(format!("route-0-{id}")))
                .ghost()
                .small()
                .h(px(24.))
                .px(px(6.))
                .border_1()
                .border_color(rgb(p.line))
                .text_color(rgb(p.muted))
                .tooltip("Return to the desktop output")
                .child(div().text_size(px(11.)).child("Off"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.send(LinkCommand::Assign(key.clone(), 0), cx)
                })),
        );
        let accent = if app.channel > 0 {
            p.channels[app.channel as usize - 1]
        } else {
            p.muted
        };
        let key = app.key.clone();
        let target = self.channel;
        div()
            .id(SharedString::from(format!("app-{id}")))
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(rgb(p.line))
            .cursor_pointer()
            .hover(|row| row.bg(rgb(p.surface)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.send(LinkCommand::Assign(key.clone(), target), cx);
                this.catalog_open = false;
                cx.notify();
            }))
            .child(
                div()
                    .size_8()
                    .flex_shrink_0()
                    .rounded_md()
                    .bg(rgb(p.surface))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(accent))
                    .child(
                        app.name
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
                    .child(div().truncate().text_size(px(13.)).child(app.name.clone()))
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(p.muted))
                            .child(status),
                    ),
            )
            .child(chips)
            .test_support()
            .into_any_element()
    }
}

/// Invisible edges and corners for resizing the undecorated window.
fn resize_handles() -> Vec<AnyElement> {
    const EDGE: f32 = 5.0;
    const CORNER: f32 = 12.0;
    let handle = |id: &'static str, edge: ResizeEdge, cursor: CursorStyle| {
        div()
            .id(id)
            .absolute()
            .cursor(cursor)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                cx.stop_propagation();
                window.start_window_resize(edge);
            })
    };
    vec![
        handle("resize-top", ResizeEdge::Top, CursorStyle::ResizeUpDown)
            .top_0().left(px(CORNER)).right(px(CORNER)).h(px(EDGE)).into_any_element(),
        handle("resize-bottom", ResizeEdge::Bottom, CursorStyle::ResizeUpDown)
            .bottom_0().left(px(CORNER)).right(px(CORNER)).h(px(EDGE)).into_any_element(),
        handle("resize-left", ResizeEdge::Left, CursorStyle::ResizeLeftRight)
            .left_0().top(px(CORNER)).bottom(px(CORNER)).w(px(EDGE)).into_any_element(),
        handle("resize-right", ResizeEdge::Right, CursorStyle::ResizeLeftRight)
            .right_0().top(px(CORNER)).bottom(px(CORNER)).w(px(EDGE)).into_any_element(),
        handle("resize-top-left", ResizeEdge::TopLeft, CursorStyle::ResizeUpLeftDownRight)
            .top_0().left_0().size(px(CORNER)).into_any_element(),
        handle("resize-top-right", ResizeEdge::TopRight, CursorStyle::ResizeUpRightDownLeft)
            .top_0().right_0().size(px(CORNER)).into_any_element(),
        handle("resize-bottom-left", ResizeEdge::BottomLeft, CursorStyle::ResizeUpRightDownLeft)
            .bottom_0().left_0().size(px(CORNER)).into_any_element(),
        handle("resize-bottom-right", ResizeEdge::BottomRight, CursorStyle::ResizeUpLeftDownRight)
            .bottom_0().right_0().size(px(CORNER)).into_any_element(),
    ]
}

impl LinkInterface {
    /// LinX's own window controls: minimize and close (close keeps LinX
    /// running in the tray, or minimizes without one).
    fn window_buttons(&self) -> Div {
        let p = self.appearance.colors();
        let control = |id: &'static str, glyph: &'static str, tip: &'static str| {
            Button::new(id)
                .ghost()
                .small()
                .size(px(29.))
                .p_0()
                .ml(px(4.))
                .rounded(px(5.))
                .text_color(rgb(p.muted))
                .tooltip(tip)
                .child(div().text_size(px(13.)).child(glyph))
        };
        div()
            .flex()
            .items_center()
            .ml(px(6.))
            .pl(px(6.))
            .border_l_1()
            .border_color(rgb(p.line))
            .child(
                control("minimize-window", "—", "Minimize")
                    .on_click(|_, window, _| window.minimize_window()),
            )
            .child(
                control("close-window", "✕", "Close (LinX keeps running)").on_click(
                    |_, window, cx| {
                        if close_requested(window, cx) {
                            window.remove_window();
                        }
                    },
                ),
            )
    }

    /// Dimmed backdrop behind a panel; clicking it closes the panel.
    fn scrim(&self, id: &'static str, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id(id)
            .absolute()
            .occlude()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .rounded(px(10.))
            .bg(rgba(if self.appearance.light { 0x1b232a40 } else { 0x0000008c }))
            .on_click(cx.listener(|this, _, _, cx| {
                this.catalog_open = false;
                this.settings = false;
                cx.notify();
            }))
    }
}

fn panel(p: &crate::appearance::Colors) -> Div {
    div()
        .absolute()
        // Block clicks from reaching the console beneath the panel.
        .occlude()
        .flex()
        .flex_col()
        .bg(rgb(p.background))
        .border_1()
        .border_color(rgb(p.line))
        .rounded(px(10.))
        .shadow_lg()
}

fn panel_header(
    title: &str,
    close_id: &'static str,
    p: &crate::appearance::Colors,
    cx: &mut Context<LinkInterface>,
) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(p.text))
                .child(title.to_owned()),
        )
        .child(
            Button::new(close_id)
                .ghost()
                .small()
                .size(px(24.))
                .p_0()
                .text_color(rgb(p.muted))
                .icon(IconName::Close)
                .tooltip("Close")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.catalog_open = false;
                    this.settings = false;
                    cx.notify();
                })),
        )
}

impl Render for LinkInterface {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.appearance.colors();
        let status = status_of(&self.view);
        if self.native_icon != Some(status) {
            crate::window_icon::update(window, status);
            self.native_icon = Some(status);
        }
        let connected = matches!(self.view.phase, Phase::Running | Phase::Retrying | Phase::Starting);
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
                let names: Vec<String> = self.view.apps_on(pair).map(|a| a.name.clone()).collect();
                let weak = cx.entity().downgrade();
                let on_level: crate::console::OnLevel =
                    std::rc::Rc::new(move |volume, mute, _window, cx| {
                        let _ = weak.update(cx, |this, cx| this.set_level(pair, volume, mute, cx));
                    });
                let level = self.level(pair).map(|level| (level, on_level));
                row = row.child(crate::console::strip(
                    pair,
                    names,
                    None,
                    if self.view.phase == Phase::Running {
                        "Choose application"
                    } else {
                        "Connect to link apps"
                    },
                    self.appearance.light,
                    p.channels[(pair - 1) as usize],
                    self.meters[(pair - 1) as usize].display,
                    level,
                    cx.listener(move |this, _, _, cx| {
                        let same = this.catalog_open && this.channel == pair;
                        this.channel = pair;
                        this.catalog_open = !same;
                        this.settings = false;
                        cx.notify();
                    }),
                ));
            }
            bank = bank.child(row);
        }
        let tool_color = if self.appearance.light { 0x43515b } else { 0xcdd1d7 };
        let mut body = div()
            .id("linx-window")
            .relative()
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .bg(rgb(p.background))
            .text_color(rgb(p.text))
            .text_size(px(14.))
            .font_family("Segoe UI")
            .border_1()
            .border_color(rgb(if self.appearance.light { 0xb9c2c2 } else { 0x3b3d41 }))
            .rounded(px(10.))
            .child(
                div()
                    .id("console-header")
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
                    .flex_shrink_0()
                    .h(px(57.))
                    .px(px(20.))
                    .flex()
                    .items_center()
                    .gap(px(9.))
                    .border_b_1()
                    .border_color(rgb(if self.appearance.light { 0xdce1dc } else { 0x2d2f32 }))
                    .child(
                        div()
                            .size(px(if self.appearance.light { 31. } else { 23. }))
                            .flex_shrink_0()
                            .rounded(px(7.))
                            .when(self.appearance.light, |e| e.bg(rgb(0x252b31)).p(px(5.)))
                            .child(
                                svg()
                                    .path(crate::ui::icon_path(status))
                                    .size_full()
                                    .text_color(crate::ui::color(status)),
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
                            .child(crate::ui::tracked("STUDIO", 9., 0.9)),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .mr(px(7.))
                            .child(div().size(px(5.)).rounded_full().bg(crate::ui::color(status)))
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(p.muted))
                                    .child(self.view.status()),
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
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        svg()
                                            .path(if connected {
                                                "console/pause.svg"
                                            } else {
                                                "console/play.svg"
                                            })
                                            .size(px(13.))
                                            .text_color(rgb(tool_color)),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .child(if connected { "Pause" } else { "Connect" }),
                                    ),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.send(
                                    if connected { LinkCommand::Pause } else { LinkCommand::Connect },
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
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.settings = !this.settings;
                                this.catalog_open = false;
                                cx.notify();
                            })),
                    )
                    .child(self.window_buttons())
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
                            .child(crate::ui::tracked("LINUX SENDS", 9., 1.17)),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(p.muted))
                            .child(if self.view.host {
                                "BEACN App connected"
                            } else if self.view.studio {
                                "BEACN App not answering"
                            } else {
                                "4 stereo channels"
                            }),
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
                            .child(svg().path("console/arrow.svg").size(px(12.)).text_color(rgb(p.muted)))
                            .child("STUDIO")
                            .child(svg().path("console/arrow.svg").size(px(12.)).text_color(rgb(p.muted)))
                            .child("STREAMING PC"),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(p.muted))
                            .child(format!(
                                "{} app{} linked",
                                self.view.apps.iter().filter(|a| a.channel > 0).count(),
                                if self.view.apps.iter().filter(|a| a.channel > 0).count() == 1 { "" } else { "s" }
                            )),
                    ),
            );
        if self.settings {
            let mut accents = div().flex().gap_2();
            for accent in crate::appearance::Accent::ALL {
                accents = accents.child(
                    Button::new(SharedString::from(format!("accent-{}", accent.label().to_lowercase())))
                        .ghost()
                        .small()
                        .label(accent.label())
                        .bg(rgb(if self.appearance.accent == accent { p.surface } else { p.background }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.appearance.accent = accent;
                            cx.notify();
                        })),
                );
            }
            body = body.child(self.scrim("settings-scrim", cx)).child(
                panel(&p)
                    .id("settings-panel")
                    .top(px(62.))
                    .right(px(14.))
                    .w(px(340.))
                    .p_4()
                    .gap_3()
                    .child(panel_header("Settings", "close-settings", &p, cx))
                    .child(
                        Switch::new("auto-connect")
                            .small()
                            .label("Connect automatically when LinX opens")
                            .checked(self.view.auto_connect)
                            .on_change(cx.listener(|this, enabled, _, cx| {
                                this.send(LinkCommand::AutoConnect(*enabled), cx)
                            })),
                    )
                    .child(
                        Switch::new("launch-at-login")
                            .small()
                            .label("Start LinX when you log in")
                            .checked(cx.global::<LinkCore>().autostart)
                            .on_change(cx.listener(|this, enabled: &bool, _, cx| {
                                let args = cx.global::<LinkCore>().login_args.clone();
                                match beacn_linx_core::autostart::set(*enabled, &args) {
                                    Ok(()) => {
                                        cx.global_mut::<LinkCore>().autostart = *enabled;
                                        this.feedback = None;
                                    }
                                    Err(error) => this.feedback = Some(error),
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(p.muted))
                            .child("Apps keep their channel when paused or restarted. Faders and mute set what this PC sends on each Link In; the streaming PC's mix is unchanged."),
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
                    .child(accents)
                    .child(
                        div().pt_2().child(
                            Button::new("quit-app")
                                .ghost()
                                .small()
                                .label("Quit LinX")
                                .tooltip("Stop routing and quit. Closing the window keeps LinX running.")
                                .on_click(|_, _, cx| {
                                    LinkCore::stop(cx);
                                    cx.quit();
                                }),
                        ),
                    ),
            );
        }
        let notice = self
            .feedback
            .clone()
            .or_else(|| self.view.message.clone())
            .or_else(|| {
                (self.view.phase == Phase::Retrying)
                    .then(|| self.view.audio_error.clone())
                    .flatten()
            })
            .or_else(|| {
                (self.view.phase == Phase::Running && self.view.studio && !self.view.audio_ready)
                    .then(|| self.view.audio_error.clone())
                    .flatten()
            });
        if let Some(message) = notice {
            body = body.child(
                div()
                    .flex_shrink_0()
                    .px_4()
                    .pb_2()
                    .text_xs()
                    .text_color(rgb(p.muted))
                    .child(beacn_linx_core::model::label(&message)),
            );
        }
        body = body.children(resize_handles());
        if self.catalog_open {
            let apps = self.filtered_apps(cx);
            let empty = apps.is_empty();
            let filtering = !self.search.read(cx).value().is_empty();
            let rows = apps.iter().map(|app| self.row(app, cx)).collect::<Vec<_>>();
            let pair = self.channel;
            let title = if pair == 1 {
                "Send to Link In".to_owned()
            } else {
                format!("Send to Link {pair} In")
            };
            body = body.child(self.scrim("applications-scrim", cx)).child(
                panel(&p)
                    .id("applications-panel")
                    .top(px(62.))
                    .left(px(14.))
                    .right(px(14.))
                    .max_h(px(470.))
                    .p_3()
                    .gap_2()
                    .child(panel_header(&title, "close-applications", &p, cx))
                    .child(
                        Input::new(&self.search)
                            .id("search-input")
                            .small()
                            .cleanable(true)
                            .prefix(gpui_kit::component::Icon::new(IconName::Search).small()),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(p.muted))
                            .child("Click an application to send it here, or pick any channel with 1–4."),
                    )
                    .child(
                        div()
                            .id("application-list")
                            .min_h(px(52.))
                            .max_h(px(340.))
                            .overflow_y_scroll()
                            .bg(rgb(p.well))
                            .rounded(px(6.))
                            .children(rows)
                            .when(empty, |el| {
                                el.child(div().p_3().text_xs().text_color(rgb(p.muted)).child(
                                    if self.view.phase != Phase::Running {
                                        "Connect LinX to see applications."
                                    } else if filtering {
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

fn open_console(cx: &mut App) {
    if let Some(window) = cx.windows().first().copied() {
        cx.activate(true);
        let _ = window.update(cx, |_, window, _| window.activate_window());
        return;
    }
    open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(788.), px(568.)),
                    cx,
                ))),
                window_min_size: Some(size(px(420.), px(380.))),
                // LinX draws its own title bar: drag the header to move,
                // its own minimize/close buttons, edges to resize.
                titlebar: Some(TitlebarOptions {
                    title: Some("LinX".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                window_decorations: Some(WindowDecorations::Client),
                window_background: WindowBackgroundAppearance::Transparent,
                // Matches io.github.linx.desktop so docks show LinX's icon.
                app_id: Some("io.github.linx".into()),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| LinkInterface::new(window, cx)),
        )
        .expect("open LinX window");
    cx.activate(true);
}

/// Tray-resident app: closing the window keeps LinX routing in the tray.
/// Without a tray host, closing the window quits so LinX never runs hidden.
pub fn run(start_hidden: bool, login_args: Vec<String>) {
    crate::shutdown::install();
    application().with_assets(crate::ui::Assets).run(move |cx| {
        init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        cx.set_quit_mode(QuitMode::Explicit);
        cx.set_global(LinkCore {
            worker: LinkWorker::spawn(),
            login_args,
            hidden_since: start_hidden.then(std::time::Instant::now),
            autostart: beacn_linx_core::autostart::enabled(),
            #[cfg(all(feature = "status-tray", target_os = "linux"))]
            tray: beacn_linx_core::tray::Tray::spawn(),
        });
        cx.on_app_quit(|cx| {
            LinkCore::stop(cx);
            async {}
        })
        .detach();
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() && !tray_available(cx) {
                LinkCore::stop(cx);
                cx.quit();
            }
        })
        .detach();
        if !start_hidden {
            open_console(cx);
        }
        cx.spawn(async move |cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                let quit = cx.update(|cx| {
                    if crate::shutdown::requested() {
                        return true;
                    }
                    tray_tick(cx)
                });
                if quit {
                    let _ = cx.update(|cx| {
                        LinkCore::stop(cx);
                        cx.quit();
                    });
                    break;
                }
            }
        })
        .detach();
    });
}

/// Window close: true lets the window close (LinX stays in the tray);
/// without a tray the window is minimized instead so routing continues.
pub(crate) fn close_requested(window: &mut Window, cx: &mut App) -> bool {
    if tray_available(cx) {
        return true;
    }
    window.minimize_window();
    false
}

fn tray_available(cx: &App) -> bool {
    #[cfg(all(feature = "status-tray", target_os = "linux"))]
    {
        cx.global::<LinkCore>().tray.available() == Some(true)
    }
    #[cfg(not(all(feature = "status-tray", target_os = "linux")))]
    {
        let _ = cx;
        false
    }
}

/// Started hidden: stay in the tray; without a tray host (or if none has
/// answered after a few seconds), show the window minimized instead.
fn reveal_if_no_tray(cx: &mut App) {
    let Some(since) = cx.global::<LinkCore>().hidden_since else {
        return;
    };
    let tray = {
        #[cfg(all(feature = "status-tray", target_os = "linux"))]
        {
            cx.global::<LinkCore>().tray.available()
        }
        #[cfg(not(all(feature = "status-tray", target_os = "linux")))]
        {
            Some(false)
        }
    };
    match tray {
        Some(true) => cx.global_mut::<LinkCore>().hidden_since = None,
        Some(false) => {}
        None if since.elapsed() < Duration::from_secs(4) => return,
        None => {}
    }
    if tray == Some(true) {
        return;
    }
    cx.global_mut::<LinkCore>().hidden_since = None;
    open_console(cx);
    if let Some(window) = cx.windows().first().copied() {
        let _ = window.update(cx, |_, window, _| window.minimize_window());
    }
}

/// Mirror status into the tray and handle its menu. True requests Quit.
fn tray_tick(cx: &mut App) -> bool {
    reveal_if_no_tray(cx);
    #[cfg(all(feature = "status-tray", target_os = "linux"))]
    {
        use beacn_linx_core::tray;
        let core = cx.global::<LinkCore>();
        let view = core.worker.view();
        core.tray.update(status_of(&view), view.phase == Phase::Running);
        let actions = core.tray.actions();
        if actions & tray::STOP != 0 {
            let _ = core.worker.send(LinkCommand::Pause);
        }
        if actions & tray::QUIT != 0 {
            return true;
        }
        if actions & tray::SHOW != 0 {
            open_console(cx);
        }
        // The tray host went away while the window was closed: quit rather
        // than keep running invisibly.
        if cx.windows().is_empty() && cx.global::<LinkCore>().tray.available() == Some(false) {
            return true;
        }
    }
    #[cfg(not(all(feature = "status-tray", target_os = "linux")))]
    let _ = cx;
    false
}
