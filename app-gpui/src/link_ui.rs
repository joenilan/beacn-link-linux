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
        Disableable, IconName, Sizable, Theme, ThemeMode,
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
    /// Single-instance guard; later launches ask through it to show.
    instance: Option<crate::single_instance::Instance>,
    pub(crate) updates: crate::updates::Updates,
    /// A newer version announced this session and not yet looked at.
    update_unseen: bool,
    /// Sound outputs hidden from the Sound output panel and tray menu.
    pub(crate) hidden_outputs: std::collections::BTreeSet<String>,
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
    pub(crate) updates_open: bool,
    /// The "Sound output" panel (desktop default output) is open.
    pub(crate) outputs_open: bool,
    /// The Sound output panel shows every output with hide/show buttons.
    pub(crate) outputs_edit: bool,
    pub(crate) catalog_open: bool,
    /// Rows of the open application picker, in a fixed order: applications
    /// that start or stop playing while it is open never move a row.
    picker: Vec<LinkApp>,
    /// Channel the picker order was built for.
    picker_for: Option<u8>,
    pub(crate) appearance: crate::appearance::Appearance,
    pub(crate) feedback: Option<String>,
    meters: [ChannelMeter; 4],
    /// What the Studio sends back on each pair (Windows: Link In 1..4).
    out_meters: [ChannelMeter; 4],
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
            updates_open: false,
            outputs_open: false,
            outputs_edit: false,
            catalog_open: false,
            picker: Vec::new(),
            picker_for: None,
            appearance: Default::default(),
            feedback: None,
            meters: Default::default(),
            out_meters: Default::default(),
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
        for (index, meter) in self.out_meters.iter_mut().enumerate() {
            let slot = next.out_meters[index].as_ref();
            let key = slot.map(|s| std::sync::Arc::as_ptr(s) as usize);
            if key != meter.slot {
                *meter = ChannelMeter {
                    slot: key,
                    ..Default::default()
                };
            }
            let reading = slot.and_then(|s| s.read());
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

    /// Keep the picker's rows in step with the applications without
    /// reordering them while it is open.
    fn sync_picker(&mut self) {
        if !self.catalog_open {
            self.picker.clear();
            self.picker_for = None;
        } else if self.picker_for != Some(self.channel) {
            self.picker = picker_order(&self.view.apps, self.channel);
            self.picker_for = Some(self.channel);
        } else {
            merge_picker(&mut self.picker, &self.view.apps);
        }
    }

    pub(crate) fn filtered_apps(&self, cx: &App) -> Vec<LinkApp> {
        let filter = self.search.read(cx).value().to_lowercase();
        let rows = if self.picker_for.is_some() {
            self.picker.clone()
        } else {
            picker_order(&self.view.apps, self.channel)
        };
        rows.into_iter()
            .filter(|a| filter.is_empty() || a.name.to_lowercase().contains(&filter))
            .collect()
    }

    /// A closed application has no stream to report its new channel; show
    /// the choice LinX will apply when it plays again.
    fn picked(&mut self, key: &str, channel: u8) {
        if let Some(row) = self.picker.iter_mut().find(|a| a.key == key && a.streams == 0) {
            row.channel = channel;
            row.remembered = (channel > 0).then_some(channel);
        }
    }

    fn row(&self, app: &LinkApp, cx: &mut Context<Self>) -> AnyElement {
        let p = self.appearance.colors();
        let id = key_id(&app.key);
        let status = match (app.channel, app.running, app.streams) {
            (0, _, 0) => "Desktop output · closed".to_owned(),
            (n, _, 0) => format!("{} · closed", crate::console::link_name(n)),
            (0, true, _) => "Playing · desktop output".to_owned(),
            (0, false, _) => "Desktop output".to_owned(),
            (n, true, _) => format!("{} · playing", crate::console::link_name(n)),
            (n, false, _) => crate::console::link_name(n),
        };
        // Opened from a channel's box (target 1..4): one action per row, for
        // that channel. Opened from Manage (target 0): every channel, per row.
        let target = self.channel;
        let mut chips = div().flex().items_center().gap(px(4.));
        if target == 0 {
            for channel in [1, 2, 3, 4, 0u8] {
                let key = app.key.clone();
                let current = app.channel == channel;
                let color = if channel == 0 { p.text } else { p.channels[channel as usize - 1] };
                chips = chips.child(
                    Button::new(SharedString::from(format!("route-{channel}-{id}")))
                        .ghost()
                        .small()
                        .h(px(24.))
                        .when(channel > 0, |b| b.w(px(26.)).p_0())
                        .when(channel == 0, |b| b.px(px(6.)))
                        .border_1()
                        .border_color(rgb(if current { color } else { p.line }))
                        .bg(rgb(if current { p.surface } else { p.well }))
                        .text_color(rgb(if current { color } else { p.muted }))
                        .tooltip(if channel == 0 {
                            "Return to the desktop output".to_owned()
                        } else {
                            format!("Send to {}", crate::console::link_name(channel))
                        })
                        .child(div().text_size(px(11.)).child(if channel == 0 {
                            "Off".to_owned()
                        } else {
                            channel.to_string()
                        }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.picked(&key, channel);
                            this.send(LinkCommand::Assign(key.clone(), channel), cx)
                        })),
                );
            }
        } else {
            let key = app.key.clone();
            let here = app.channel == target;
            let (channel, label, tooltip) = if here {
                (0, "Remove", "Return to the desktop output".to_owned())
            } else if app.channel > 0 {
                (target, "Move here", format!("Move to {}", crate::console::link_name(target)))
            } else {
                (target, "Add", format!("Send to {}", crate::console::link_name(target)))
            };
            let color = p.channels[target as usize - 1];
            chips = chips.child(
                Button::new(SharedString::from(format!("route-{channel}-{id}")))
                    .ghost()
                    .small()
                    .h(px(24.))
                    .px(px(8.))
                    .border_1()
                    .border_color(rgb(if here { p.line } else { color }))
                    .bg(rgb(if here { p.well } else { p.surface }))
                    .text_color(rgb(if here { p.muted } else { color }))
                    .tooltip(tooltip)
                    .child(div().text_size(px(11.)).child(label))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.picked(&key, channel);
                        this.send(LinkCommand::Assign(key.clone(), channel), cx)
                    })),
            );
        }
        let accent = if app.channel > 0 {
            p.channels[app.channel as usize - 1]
        } else {
            p.muted
        };
        let key = app.key.clone();
        div()
            .id(SharedString::from(format!("app-{id}")))
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(rgb(p.line))
            .when(target > 0, |row| {
                row.cursor_pointer()
                    .hover(|row| row.bg(rgb(p.surface)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.send(LinkCommand::Assign(key.clone(), target), cx);
                        this.catalog_open = false;
                        cx.notify();
                    }))
            })
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

/// The picker's order when it opens: applications on the chosen channel
/// first, then playing ones, then by name. The manager (channel 0) lists
/// linked applications first.
fn picker_order(apps: &[LinkApp], channel: u8) -> Vec<LinkApp> {
    let mut rows = apps.to_vec();
    if channel == 0 {
        rows.sort_by_key(|a| (a.channel == 0, !a.running, a.name.to_lowercase()));
    } else {
        rows.sort_by_key(|a| (a.channel != channel, !a.running, a.name.to_lowercase()));
    }
    rows
}

/// Update rows in place; an application that closed keeps its row (marked
/// closed, still assignable), and one that appeared goes to the end.
fn merge_picker(rows: &mut Vec<LinkApp>, apps: &[LinkApp]) {
    for row in rows.iter_mut() {
        match apps.iter().find(|a| a.key == row.key) {
            Some(app) => *row = app.clone(),
            None if row.streams > 0 => {
                row.running = false;
                row.streams = 0;
            }
            None => {}
        }
    }
    let mut added: Vec<LinkApp> = apps
        .iter()
        .filter(|a| !rows.iter().any(|r| r.key == a.key))
        .cloned()
        .collect();
    added.sort_by_key(|a| a.name.to_lowercase());
    rows.extend(added);
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
                this.updates_open = false;
                this.outputs_open = false;
                cx.notify();
            }))
    }
}

impl LinkInterface {
    /// The desktop's default output. Applications on a LinX channel stay
    /// there; everything else follows the choice.
    fn outputs_panel(&self, p: &crate::appearance::Colors, cx: &mut Context<Self>) -> Stateful<Div> {
        let mut list = div()
            .id("output-list")
            .max_h(px(360.))
            .overflow_y_scroll()
            .bg(rgb(p.well))
            .rounded(px(6.))
            .flex()
            .flex_col();
        let edit = self.outputs_edit;
        let hidden = cx.global::<LinkCore>().hidden_outputs.clone();
        let mut shown = 0;
        for output in &self.view.outputs {
            let current = self.view.default_output.as_deref() == Some(output.name.as_str());
            let is_hidden = hidden.contains(&output.name) && !current;
            if is_hidden && !edit {
                continue;
            }
            shown += 1;
            let name = output.name.clone();
            let id = key_id(&output.name);
            let mut row = div()
                .id(SharedString::from(format!("output-{id}")))
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(rgb(p.line))
                .child(
                    div()
                        .size(px(8.))
                        .flex_shrink_0()
                        .rounded_full()
                        .border_1()
                        .border_color(rgb(if current { p.channels[0] } else { p.line }))
                        .when(current, |dot| dot.bg(rgb(p.channels[0]))),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(13.))
                        .text_color(rgb(if current {
                            p.text
                        } else if is_hidden {
                            p.line
                        } else {
                            p.muted
                        }))
                        .child(output.description.clone()),
                );
            if edit {
                let key = output.name.clone();
                row = row.child(
                    Button::new(SharedString::from(format!("output-visibility-{id}")))
                        .ghost()
                        .small()
                        .h(px(22.))
                        .px(px(8.))
                        .border_1()
                        .border_color(rgb(p.line))
                        .text_color(rgb(p.muted))
                        .disabled(current)
                        .tooltip(if current {
                            "The output in use is always listed"
                        } else if is_hidden {
                            "List this output again"
                        } else {
                            "Hide this output from LinX's lists"
                        })
                        .child(div().text_size(px(10.)).child(if is_hidden { "Show" } else { "Hide" }))
                        .on_click(cx.listener(move |_, _, _, cx| {
                            cx.stop_propagation();
                            let core = cx.global_mut::<LinkCore>();
                            if !core.hidden_outputs.remove(&key) {
                                core.hidden_outputs.insert(key.clone());
                            }
                            let _ = beacn_linx_core::link::HiddenOutputs::user().save(&core.hidden_outputs);
                            cx.notify();
                        })),
                );
            } else {
                row = row
                    .cursor_pointer()
                    .hover(|row| row.bg(rgb(p.surface)))
                    .when(current, |row| {
                        row.child(div().text_size(px(10.)).text_color(rgb(p.muted)).child("In use"))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.send(LinkCommand::DefaultOutput(name.clone()), cx);
                        this.outputs_open = false;
                        cx.notify();
                    }));
            }
            list = list.child(row.test_support());
        }
        let hidden_count = self.view.outputs.len() - shown;
        panel(p)
            .id("outputs-panel")
            .top(px(62.))
            .right(px(14.))
            .w(px(340.))
            .p_3()
            .gap_2()
            .child(panel_header("Sound output", "close-outputs", p, cx))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_size(px(10.)).text_color(rgb(p.muted)).child(if edit {
                        "Hide outputs you never use. Hiding only affects LinX's lists.".to_owned()
                    } else if hidden_count > 0 {
                        format!("{hidden_count} hidden")
                    } else {
                        String::new()
                    }))
                    .child(
                        Button::new("outputs-edit")
                            .ghost()
                            .small()
                            .h(px(22.))
                            .px(px(8.))
                            .text_color(rgb(p.muted))
                            .child(div().text_size(px(10.)).child(if edit { "Done" } else { "Edit" }))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.outputs_edit = !this.outputs_edit;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(rgb(p.muted))
                    .child(if self.view.local_output {
                        "Everything plays here, assigned applications included. Choose BEACN Studio (LinX) to return each one to its Link channel."
                    } else {
                        "BEACN Studio (LinX) keeps each application on its Link channel. Another output plays everything there."
                    }),
            )
            .child(list)
    }

    fn updates_panel(&self, p: &crate::appearance::Colors, cx: &mut Context<Self>) -> Stateful<Div> {
        use crate::updates::{InstallKind, Phase};
        let update = cx.global::<LinkCore>().updates.view();
        let latest = update.latest.clone().unwrap_or_default();
        let mut notes = div()
            .id("update-notes")
            .max_h(px(220.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_1()
            .p_3()
            .rounded(px(6.))
            .bg(rgb(p.well))
            .text_size(px(11.))
            .text_color(rgb(p.text));
        for line in update.notes.iter().filter(|l| !l.trim().is_empty()) {
            notes = notes.child(line.clone());
        }
        let action = match update.phase {
            Phase::Available | Phase::Failed if update.ineligible.is_none() && update.latest.is_some() => {
                Some(Button::new("update-download").label("Download").on_click(cx.listener(
                    |_, _, _, cx| {
                        cx.global_mut::<LinkCore>().updates.start_download();
                        cx.notify();
                    },
                )))
            }
            Phase::Downloading { percent } => Some(
                Button::new("update-download")
                    .label(format!("Downloading… {percent}%"))
                    .disabled(true),
            ),
            Phase::Ready => Some(
                Button::new("update-install")
                    .label("Install")
                    .when(matches!(update.kind, InstallKind::Rpm | InstallKind::Deb), |b| {
                        b.tooltip("Your system asks for your password to install the package.")
                    })
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.global_mut::<LinkCore>().updates.start_install();
                        cx.notify();
                    })),
            ),
            Phase::Installing => Some(Button::new("update-install").label("Installing…").disabled(true)),
            Phase::Installed => Some(
                Button::new("update-restart")
                    .label("Restart LinX")
                    .tooltip("Routing stops for a moment while LinX restarts.")
                    .on_click(cx.listener(|_, _, window, cx| {
                        // Wayland: ask for an activation token while this
                        // click still counts, so the new LinX opens in front
                        // (KWin otherwise opens it behind other windows).
                        #[cfg(target_os = "linux")]
                        let token = gpui_linux::request_launch_activation_token();
                        window.activate_window();
                        cx.spawn(async move |this, cx| {
                            let mut activation = None;
                            #[cfg(target_os = "linux")]
                            for _ in 0..25 {
                                if let Ok(t) = token.try_recv() {
                                    activation = Some(t);
                                    break;
                                }
                                cx.background_executor()
                                    .timer(Duration::from_millis(20))
                                    .await;
                            }
                            let _ = this.update(cx, |this, cx| {
                                let args: Vec<String> = std::env::args().skip(1).collect();
                                match cx.global::<LinkCore>().updates.relaunch(&args, activation) {
                                    Ok(()) => {
                                        LinkCore::stop(cx);
                                        cx.quit();
                                    }
                                    Err(error) => {
                                        this.feedback = Some(error);
                                        cx.notify();
                                    }
                                }
                            });
                        })
                        .detach();
                    })),
            ),
            _ => None,
        };
        panel(p)
            .id("updates-panel")
            .top(px(62.))
            .right(px(14.))
            .w(px(380.))
            .p_4()
            .gap_3()
            .child(panel_header("Update LinX", "close-updates", p, cx))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(p.text))
                    .child(match update.phase {
                        Phase::Installed => format!("LinX {latest} is installed. Restart LinX to use it."),
                        _ if update.latest.is_some() => {
                            format!("LinX {latest} is available. You have {}.", update.current)
                        }
                        _ => update_status(&update),
                    }),
            )
            .when(!update.notes.is_empty(), |panel| panel.child(notes))
            .when_some(update.ineligible, |panel, reason| {
                panel.child(div().text_size(px(11.)).text_color(rgb(p.muted)).child(reason))
            })
            .when_some(update.error.clone(), |panel, error| {
                panel.child(div().text_size(px(11.)).text_color(rgb(0xe58a8a)).child(error))
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        Button::new("update-release-page")
                            .ghost()
                            .small()
                            .label("Release page")
                            .on_click(|_, _, cx| cx.open_url(crate::updates::RELEASE_PAGE)),
                    )
                    .children(action.map(|b| b.small())),
            )
    }
}

/// Header button text while an update needs attention.
fn update_badge(update: &crate::updates::UpdateView) -> Option<String> {
    use crate::updates::Phase;
    let latest = update.latest.as_deref()?;
    Some(match update.phase {
        Phase::Available | Phase::Ready => format!("Update {latest}"),
        Phase::Downloading { percent } => format!("Downloading {percent}%"),
        Phase::Installing => "Installing…".to_owned(),
        Phase::Installed => "Restart to update".to_owned(),
        Phase::Failed => "Update failed".to_owned(),
        _ => return None,
    })
}

fn update_status(update: &crate::updates::UpdateView) -> String {
    use crate::updates::Phase;
    match (&update.phase, &update.latest) {
        (Phase::Checking, _) => "Checking for updates…".to_owned(),
        (Phase::UpToDate, _) => format!("LinX {} is up to date.", update.current),
        (Phase::Failed, _) => update.error.clone().unwrap_or_else(|| "Update check failed.".to_owned()),
        (_, Some(latest)) => format!("LinX {latest} is available."),
        _ => format!("LinX {}", update.current),
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
                    this.updates_open = false;
                    this.outputs_open = false;
                    cx.notify();
                })),
        )
}

impl Render for LinkInterface {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_picker();
        let p = self.appearance.colors();
        let status = status_of(&self.view);
        if self.native_icon != Some(status) {
            crate::window_icon::update(window, status);
            self.native_icon = Some(status);
        }
        let connected = matches!(self.view.phase, Phase::Running | Phase::Retrying | Phase::Starting);
        let narrow = f32::from(console_width(window)) < 700.;
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
                    self.view
                        .link_out
                        .complete()
                        .then_some(self.out_meters[(pair - 1) as usize].display),
                    cx.listener(move |this, _, _, cx| {
                        let same = this.catalog_open && this.channel == pair;
                        this.channel = pair;
                        this.catalog_open = !same;
                        this.settings = false;
                        this.updates_open = false;
                        this.outputs_open = false;
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
            .map(|body| console_corners(body, window))
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
                    .when(crate::updates::dev_build(), |row| {
                        row.child(
                            div()
                                .id("dev-badge")
                                .ml(px(8.))
                                .px(px(6.))
                                .py(px(1.))
                                .rounded(px(4.))
                                .bg(rgb(0xb7791f))
                                .text_color(rgb(0x1b1408))
                                .text_size(px(9.))
                                .font_weight(FontWeight::BOLD)
                                .child("DEV")
                                .test_support(),
                        )
                    })
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
                    .when_some(update_badge(&cx.global::<LinkCore>().updates.view()), |row, label| {
                        let unseen = cx.global::<LinkCore>().update_unseen;
                        row.child(
                            Button::new("update")
                                .ghost()
                                .small()
                                .h(px(29.))
                                .px(px(9.))
                                .mr(px(7.))
                                .border_1()
                                .border_color(rgb(if unseen { p.channels[0] } else { p.line }))
                                .rounded(px(5.))
                                .text_color(rgb(p.channels[0]))
                                .child(div().text_size(px(12.)).child(label))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    cx.global_mut::<LinkCore>().update_unseen = false;
                                    this.updates_open = !this.updates_open;
                                    this.outputs_open = false;
                                    this.settings = false;
                                    this.catalog_open = false;
                                    cx.notify();
                                })),
                        )
                    })
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
                    .child({
                        let current = self
                            .view
                            .default_output
                            .as_ref()
                            .and_then(|d| self.view.outputs.iter().find(|o| &o.name == d))
                            .map(|o| o.description.clone())
                            .unwrap_or_else(|| "Unknown".into());
                        Button::new("outputs")
                            .ghost()
                            .small()
                            .ml(px(7.))
                            .size(px(29.))
                            .p_0()
                            .border_1()
                            .border_color(rgb(p.line))
                            .rounded(px(5.))
                            .text_color(rgb(tool_color))
                            .disabled(self.view.outputs.is_empty())
                            .tooltip(format!("Sound output: {current}"))
                            .child(
                                svg()
                                    .path("console/speaker.svg")
                                    .size(px(13.))
                                    .text_color(rgb(tool_color)),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.outputs_edit = false;
                                this.outputs_open = !this.outputs_open;
                                this.settings = false;
                                this.catalog_open = false;
                                this.updates_open = false;
                                cx.notify();
                            }))
                    })
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
                                this.updates_open = false;
                                this.outputs_open = false;
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
                    .child({
                        let linked = self.view.apps.iter().filter(|a| a.channel > 0).count();
                        Button::new("manage-apps")
                            .ghost()
                            .small()
                            .h(px(22.))
                            .px(px(8.))
                            .text_color(rgb(p.muted))
                            .tooltip("All applications and their channels")
                            .child(div().text_size(px(10.)).child(format!(
                                "{linked} app{} linked · Manage",
                                if linked == 1 { "" } else { "s" }
                            )))
                            .on_click(cx.listener(|this, _, _, cx| {
                                let same = this.catalog_open && this.channel == 0;
                                this.channel = 0;
                                this.catalog_open = !same;
                                this.settings = false;
                                this.updates_open = false;
                                this.outputs_open = false;
                                cx.notify();
                            }))
                    }),
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
                    .child({
                        let update = cx.global::<LinkCore>().updates.view();
                        let checking = update.phase == crate::updates::Phase::Checking;
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                Switch::new("update-checks")
                                    .small()
                                    .label("Check for updates automatically")
                                    .checked(update.mode == crate::updates::UpdateMode::Notify)
                                    .on_change(cx.listener(|this, enabled: &bool, _, cx| {
                                        let mode = if *enabled {
                                            crate::updates::UpdateMode::Notify
                                        } else {
                                            crate::updates::UpdateMode::Off
                                        };
                                        this.feedback =
                                            cx.global_mut::<LinkCore>().updates.set_mode(mode).err();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(rgb(p.muted))
                                            .child(update_status(&update)),
                                    )
                                    .child(
                                        Button::new("check-updates")
                                            .ghost()
                                            .small()
                                            .label(if checking { "Checking…" } else { "Check now" })
                                            .disabled(checking)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                cx.global_mut::<LinkCore>().updates.check_now();
                                                this.feedback = None;
                                                cx.notify();
                                            })),
                                    ),
                            )
                    })
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(p.muted))
                            .child("Apps keep their channel when paused or restarted. Faders and mute set what this PC sends on each Link Out; the streaming PC's mix is unchanged."),
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
        if self.updates_open {
            body = body
                .child(self.scrim("updates-scrim", cx))
                .child(self.updates_panel(&p, cx));
        }
        if self.outputs_open {
            body = body
                .child(self.scrim("outputs-scrim", cx))
                .child(self.outputs_panel(&p, cx));
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
            let title = if pair == 0 {
                "Applications".to_owned()
            } else {
                format!("Send to {}", crate::console::link_name(pair))
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
                            .child(if pair == 0 {
                                "Choose each application's channel with 1–4, or Off for the desktop output."
                            } else {
                                "Click an application to send it to this channel."
                            }),
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
    open_console_window(
            WindowOptions {
                // The console is designed at 788x568; the window adds the
                // client-side shadow margin around it so nothing scrolls.
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(788. + 2. * FRAME_SHADOW), px(568. + 2. * FRAME_SHADOW)),
                    cx,
                ))),
                window_min_size: Some(size(
                    px(420. + 2. * FRAME_SHADOW),
                    px(380. + 2. * FRAME_SHADOW),
                )),
                // LinX draws its own title bar: drag the header to move,
                // its own minimize/close buttons, edges to resize.
                titlebar: Some(TitlebarOptions {
                    title: Some(if crate::updates::dev_build() { "LinX Dev" } else { "LinX" }.into()),
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

/// Like `gpui_kit::open_window`, but the root surface stays transparent: the
/// client-side frame is rounded (see vendor/README-LINX-PATCHES.md), and only
/// the console body paints, so no square background shows at the corners.
fn open_console_window<V: Render>(
    options: WindowOptions,
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> anyhow::Result<WindowHandle<base::Root>> {
    cx.open_window(options, |window, cx| {
        let view = build(window, cx);
        cx.new(|cx| base::Root::new(view, window, cx).bg(transparent_black()))
    })
}

/// Round the console body inside the frame's 1px border, square on the
/// corners the compositor has tiled (where the frame is square too).
fn console_corners(body: Stateful<Div>, window: &Window) -> Stateful<Div> {
    let tiling = match window.window_decorations() {
        Decorations::Client { tiling } => tiling,
        Decorations::Server => return body.rounded(px(FRAME_RADIUS)),
    };
    let radius = px(FRAME_RADIUS - 1.);
    body.when(!(tiling.top || tiling.left), |b| b.rounded_tl(radius))
        .when(!(tiling.top || tiling.right), |b| b.rounded_tr(radius))
        .when(!(tiling.bottom || tiling.left), |b| b.rounded_bl(radius))
        .when(!(tiling.bottom || tiling.right), |b| b.rounded_br(radius))
}

/// Corner radius of the vendored gpui-component window frame (BORDER_RADIUS).
const FRAME_RADIUS: f32 = 10.0;

/// GPUI Kit's window border draws a client-side shadow margin of this size on
/// every side on Linux (gpui-component `window_border` SHADOW_SIZE) and sets
/// the platform inset to match; elsewhere it draws none.
#[cfg(target_os = "linux")]
const FRAME_SHADOW: f32 = 20.0;
#[cfg(not(target_os = "linux"))]
const FRAME_SHADOW: f32 = 0.0;

/// Width available to the console inside the window's shadow margin.
pub(crate) fn console_width(window: &Window) -> Pixels {
    let inset = window.client_inset().unwrap_or(px(0.));
    window.viewport_size().width - inset * 2.
}
#[cfg_attr(not(feature = "native-control"), allow(dead_code))]
pub(crate) fn console_height(window: &Window) -> Pixels {
    let inset = window.client_inset().unwrap_or(px(0.));
    window.viewport_size().height - inset * 2.
}

/// Tray-resident app: closing the window keeps LinX routing in the tray.
/// Without a tray host, closing the window quits so LinX never runs hidden.
pub fn run(
    start_hidden: bool,
    login_args: Vec<String>,
    instance: Option<crate::single_instance::Instance>,
) {
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
            instance,
            updates: crate::updates::Updates::load(),
            update_unseen: false,
            hidden_outputs: beacn_linx_core::link::HiddenOutputs::user().load(),
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
                    let core = cx.global_mut::<LinkCore>();
                    let changed = core.updates.poll();
                    if core.updates.take_announcement().is_some() {
                        core.update_unseen = true;
                    }
                    if changed {
                        cx.refresh_windows();
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
    // Another launch of LinX: show this window instead (a hidden login
    // launch only confirms LinX is running).
    let requests = cx
        .global::<LinkCore>()
        .instance
        .as_ref()
        .map(|i| i.requests())
        .unwrap_or_default();
    if let Some(token) = requests.iter().find_map(|r| match r {
        crate::single_instance::Request::Show(token) => Some(token.clone()),
        crate::single_instance::Request::Hidden => None,
    }) {
        if let Some(token) = token {
            gpui_linux::set_pending_activation_token(token);
        }
        open_console(cx);
    }
    #[cfg(all(feature = "status-tray", target_os = "linux"))]
    {
        use beacn_linx_core::tray;
        let core = cx.global::<LinkCore>();
        let view = core.worker.view();
        core.tray.update(status_of(&view), view.phase == Phase::Running);
        core.tray.set_outputs(
            view.outputs
                .iter()
                .filter(|o| {
                    !core.hidden_outputs.contains(&o.name)
                        || view.default_output.as_deref() == Some(o.name.as_str())
                })
                .map(|o| (o.name.clone(), o.description.clone()))
                .collect(),
            view.default_output.clone(),
        );
        let actions = core.tray.actions();
        if actions & tray::STOP != 0 {
            let _ = core.worker.send(LinkCommand::Pause);
        }
        if actions & tray::OUTPUT != 0 {
            if let Some(name) = core.tray.take_output() {
                let _ = core.worker.send(LinkCommand::DefaultOutput(name));
            }
        }
        if actions & tray::QUIT != 0 {
            return true;
        }
        if actions & tray::SHOW != 0 {
            // Wayland: raise with the click's activation token, not just
            // request attention.
            if let Some(token) = cx.global::<LinkCore>().tray.take_activation_token() {
                gpui_linux::set_pending_activation_token(token);
            }
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

#[cfg(test)]
mod tests {
    use super::{merge_picker, picker_order, LinkApp};

    fn app(key: &str, channel: u8, running: bool) -> LinkApp {
        LinkApp {
            key: key.into(),
            name: key.into(),
            channel,
            remembered: (channel > 0).then_some(channel),
            running,
            streams: 1,
        }
    }

    #[test]
    fn manager_lists_linked_applications_first() {
        let rows = picker_order(&[app("Brave", 0, true), app("Discord", 3, false), app("Spotify", 2, true)], 0);
        let names: Vec<_> = rows.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Spotify", "Discord", "Brave"]);
    }

    #[test]
    fn picker_rows_keep_their_place_while_apps_come_and_go() {
        let mut rows = picker_order(&[app("Brave", 0, true), app("Discord", 2, false), app("Spotify", 0, false)], 2);
        let keys = |rows: &[LinkApp]| rows.iter().map(|a| a.key.clone()).collect::<Vec<_>>();
        assert_eq!(keys(&rows), ["Discord", "Brave", "Spotify"]);
        // Brave stops, Spotify starts playing, Firefox appears.
        merge_picker(&mut rows, &[app("Discord", 2, false), app("Spotify", 0, true), app("Firefox", 0, true)]);
        assert_eq!(keys(&rows), ["Discord", "Brave", "Spotify", "Firefox"]);
        assert_eq!((rows[1].streams, rows[1].running), (0, false));
        assert!(rows[2].running);
        // Brave plays again: same row, live data.
        merge_picker(&mut rows, &[app("Brave", 3, true), app("Discord", 2, false)]);
        assert_eq!(keys(&rows), ["Discord", "Brave", "Spotify", "Firefox"]);
        assert_eq!((rows[1].channel, rows[1].streams), (3, 1));
    }
}
