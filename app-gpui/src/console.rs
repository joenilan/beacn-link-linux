use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::appearance::Appearance;
use std::{cell::Cell, rc::Rc};

const STRIP_HEIGHT: f32 = 420.0;
const FADER_HEIGHT: f32 = 174.0;
const CAP_HEIGHT: f32 = 13.0;
/// Fader range: top is 0 dB, the bottom stop is silence.
pub const FADER_FLOOR_DB: f32 = -60.0;

/// A channel's send level for an interactive fader.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Level {
    /// Gain in dB (negative infinity when silent).
    pub db: f32,
    pub muted: bool,
}
impl Level {
    pub fn from_linear(volume: f32, muted: bool) -> Self {
        let db = if volume <= 0.0 {
            f32::NEG_INFINITY
        } else {
            (20.0 * volume.log10()).min(0.0)
        };
        Self { db, muted }
    }
    pub fn linear(db: f32) -> f32 {
        if db <= FADER_FLOOR_DB {
            0.0
        } else {
            10f32.powf(db.min(0.0) / 20.0)
        }
    }
    fn label(self) -> String {
        if self.db <= FADER_FLOOR_DB {
            "−∞".into()
        } else if self.db > -0.05 {
            "0.0".into()
        } else {
            format!("−{:.1}", -self.db)
        }
    }
}
/// Fader callback: (new linear volume, new mute), each optional.
pub type OnLevel = Rc<dyn Fn(Option<f32>, Option<bool>, &mut Window, &mut App)>;

fn cap_offset(db: f32) -> f32 {
    let travel = FADER_HEIGHT - CAP_HEIGHT;
    let fraction = ((db.max(FADER_FLOOR_DB) - FADER_FLOOR_DB) / -FADER_FLOOR_DB).clamp(0.0, 1.0);
    travel * (1.0 - fraction)
}
fn db_at(track: Bounds<Pixels>, y: Pixels) -> f32 {
    let travel = f32::from(track.size.height) - CAP_HEIGHT;
    let offset = f32::from(y - track.origin.y) - CAP_HEIGHT / 2.0;
    let fraction = (1.0 - offset / travel.max(1.0)).clamp(0.0, 1.0);
    if fraction <= 0.005 {
        f32::NEG_INFINITY
    } else {
        FADER_FLOOR_DB * (1.0 - fraction)
    }
}

/// Render one static Console strip. The application well owns the only click
/// handler; the disabled fader and mute decoration cannot trigger selection.
pub fn strip(
    pair: u8,
    names: Vec<String>,
    pending_name: Option<String>,
    empty_label: &'static str,
    light: bool,
    accent: u32,
    meter: Option<crate::meter::Display>,
    level: Option<(Level, OnLevel)>,
    choose: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let colors = Appearance {
        light,
        ..Appearance::default()
    }
    .colors();
    let dark = !light;
    let title = match pair {
        1 => "Game",
        2 => "Voice",
        3 => "Music",
        4 => "Browser",
        _ => "Channel",
    };
    let route = if pending_name.is_some() {
        "Waiting for audio".to_string()
    } else if names.is_empty() {
        "Not linked".to_string()
    } else if pair == 1 {
        "Link In".to_string()
    } else {
        format!("Link {pair} In")
    };
    let well_id = if pair == 1 {
        "applications".to_string()
    } else {
        format!("applications-{pair}")
    };
    let application = match names.len() {
        0 => pending_name.clone().unwrap_or_else(|| empty_label.into()),
        1 => names[0].clone(),
        n => format!("{} +{}", names[0], n - 1),
    };
    let glyph = names
        .first()
        .or(pending_name.as_ref())
        .and_then(|name| name.chars().next())
        .map(|ch| ch.to_uppercase().to_string())
        .unwrap_or_else(|| "+".to_string());

    let (
        base,
        top,
        middle,
        bottom,
        well,
        rail,
        rail_highlight,
        cap_top,
        cap_mid,
        cap_bottom,
        button,
        button_border,
        glyph_bg,
    ) = if dark {
        (
            0x292c30,
            0x2d3035,
            0x292c30,
            0x27292d,
            0x1c1f23,
            0x121518,
            0x292d31,
            0xdce1e5,
            0x777d84,
            0xc1c6ce,
            0x33373d,
            0x464c54,
            mix(accent, 0x24272d, 14),
        )
    } else {
        (
            colors.surface,
            colors.surface,
            colors.surface,
            colors.surface,
            colors.well,
            colors.track,
            0xffffff,
            0xf0f3f2,
            colors.cap,
            0xe0e5e3,
            colors.surface,
            colors.line,
            mix(accent, 0xffffff, 14),
        )
    };

    let mut root = div()
        .id(format!("channel-{pair}"))
        .flex_1()
        .min_w_0()
        .h(px(STRIP_HEIGHT))
        .relative()
        .flex()
        .flex_col()
        .overflow_hidden()
        .when(pair == 1, |e| e.rounded_l(px(3.)))
        .when(pair == 4, |e| e.rounded_r(px(3.)))
        .border_t_2()
        .border_color(rgb(accent))
        .px(px(13.))
        .bg(rgb(base));

    if dark {
        root = root
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(STRIP_HEIGHT * 0.45))
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(rgb(top), 0.),
                        linear_color_stop(rgb(middle), 1.),
                    )),
            )
            .child(
                div()
                    .absolute()
                    .top(px(STRIP_HEIGHT * 0.45))
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(rgb(middle), 0.),
                        linear_color_stop(rgb(bottom), 1.),
                    )),
            );
    }
    if pair > 1 {
        root = root.child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .bottom_0()
                .w(px(1.))
                .bg(rgb(if dark { 0x151719 } else { 0xbcc6c4 })),
        );
    }

    root.child(
        div()
            .relative()
            .pl(px(if pair == 1 { 0. } else { 1. }))
            .flex()
            .flex_col()
            .h_full()
            .pt(px(14.))
            .child(
                div()
                    .h(px(19.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(14.))
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .text_color(rgb(if dark { 0xe5e7e8 } else { colors.text }))
                    .child(title)
                    .child(
                        div()
                            .font_family("Consolas")
                            .text_size(px(10.))
                            .text_color(rgb(0xb1b8be))
                            .child(format!("{pair:02}")),
                    ),
            )
            .child(
                div()
                    .mt(px(5.))
                    .h(px(14.))
                    .flex()
                    .items_center()
                    .text_size(px(10.))
                    .text_color(rgb(if dark { 0xabb1b8 } else { colors.muted }))
                    .child(route),
            )
            .child({
                let block = fader_block(
                    dark,
                    level
                        .as_ref()
                        .map(|(l, _)| cap_offset(l.db))
                        .unwrap_or([16., 8., 32., 24.][(pair - 1) as usize]),
                    colors.muted,
                    rail,
                    rail_highlight,
                    cap_top,
                    cap_mid,
                    cap_bottom,
                    colors.line,
                    meter,
                );
                match &level {
                    Some((current, on_level)) => interactive_fader(pair, block, *current, on_level.clone())
                        .into_any_element(),
                    None => block.into_any_element(),
                }
            })
            .child(
                div()
                    .h(px(24.))
                    .mb(px(10.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .w(px(46.))
                            .text_right()
                            .text_size(px(18.))
                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                            .text_color(rgb(if dark { 0xe5e7e8 } else { colors.text }))
                            .child(
                                level
                                    .as_ref()
                                    .map(|(l, _)| l.label())
                                    .unwrap_or_else(|| "—".into()),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(9.))
                            .text_color(rgb(if dark { 0xa3abb4 } else { colors.muted }))
                            .child("dB"),
                    ),
            )
            .child({
                let muted = level.as_ref().is_some_and(|(l, _)| l.muted);
                let (bg, border, fg) = if muted {
                    if dark { (0x5a2427, 0xa1373d, 0xffd9db) } else { (0xf6d6d7, 0xc8484e, 0x8c1d22) }
                } else {
                    (button, button_border, if dark { 0xcdd2d8 } else { colors.muted })
                };
                let button = div()
                    .id(format!("mute-{pair}"))
                    .h(px(32.))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(rgb(border))
                    .bg(rgb(bg))
                    .text_size(px(12.))
                    .text_color(rgb(fg))
                    .child(
                        svg()
                            .path("console/speaker.svg")
                            .size(px(13.))
                            .text_color(rgb(fg)),
                    )
                    .child(if muted { "Muted" } else { "Mute" });
                match &level {
                    Some((current, on_level)) => {
                        let on_level = on_level.clone();
                        let muted = current.muted;
                        button
                            .cursor_pointer()
                            .on_click(move |_, window, cx| on_level(None, Some(!muted), window, cx))
                            .test_support()
                            .into_any_element()
                    }
                    None => button.into_any_element(),
                }
            })
            .child(
                div()
                    .id(well_id)
                    .mt(px(13.))
                    .pt(px(10.))
                    .border_t_1()
                    .border_color(rgb(if dark { 0x383b40 } else { colors.line }))
                    .cursor_pointer()
                    .on_click(choose)
                    .child(
                        div()
                            .mb(px(7.))
                            .h(px(12.))
                            .flex()
                            .items_center()
                            .text_size(px(9.))
                            .text_color(rgb(if dark { 0xa0a8b1 } else { colors.muted }))
                            .child("APPLICATION"),
                    )
                    .child(
                        div()
                            .h(px(33.))
                            .w_full()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .rounded(px(3.))
                            .px(px(7.))
                            .bg(rgb(well))
                            .text_size(px(11.))
                            .text_color(rgb(if dark { 0xe5e7e8 } else { colors.text }))
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .w(px(19.))
                                    .h(px(19.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.))
                                    .bg(rgb(glyph_bg))
                                    .text_size(px(10.))
                                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                                    .text_color(rgb(accent))
                                    .child(glyph),
                            )
                            .child(div().min_w_0().truncate().child(application)),
                    )
                    .test_support(),
            ),
    )
}

/// Drag or click the track to set the level; the wheel nudges by 1 dB.
fn interactive_fader(pair: u8, block: Div, current: Level, on_level: OnLevel) -> impl IntoElement {
    let track: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
    let set = {
        let track = track.clone();
        let on_level = on_level.clone();
        move |y: Pixels, window: &mut Window, cx: &mut App| {
            if let Some(bounds) = track.get() {
                on_level(Some(Level::linear(db_at(bounds, y))), None, window, cx);
            }
        }
    };
    let drag = set.clone();
    let stored = track.clone();
    div()
        .id(format!("fader-{pair}"))
        .cursor_pointer()
        .child(block.on_children_prepainted(move |children, _, _| {
            // Children: scale labels, fader track, meters.
            stored.set(children.get(1).copied());
        }))
        .on_mouse_down(MouseButton::Left, move |event, window, cx| {
            set(event.position.y, window, cx)
        })
        .on_mouse_move(move |event, window, cx| {
            if event.pressed_button == Some(MouseButton::Left) {
                drag(event.position.y, window, cx)
            }
        })
        .on_scroll_wheel(move |event, window, cx| {
            let dy = f32::from(event.delta.pixel_delta(px(20.)).y);
            if dy == 0.0 {
                return;
            }
            let from = if current.db.is_finite() { current.db } else { FADER_FLOOR_DB - 1.0 };
            let db = (from.max(FADER_FLOOR_DB - 1.0) + dy.signum()).min(0.0);
            on_level(Some(Level::linear(db)), None, window, cx);
        })
        .test_support()
}

#[allow(clippy::too_many_arguments)]
fn fader_block(
    dark: bool,
    cap_offset: f32,
    muted: u32,
    rail: u32,
    rail_highlight: u32,
    cap_top: u32,
    cap_mid: u32,
    cap_bottom: u32,
    cap_border: u32,
    meter: Option<crate::meter::Display>,
) -> Div {
    div()
        .mt(px(22.))
        .mb(px(16.))
        .h(px(FADER_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(13.))
        .child(
            div()
                .h(px(FADER_HEIGHT))
                .w(px(15.))
                .py(px(2.))
                .flex()
                .flex_col()
                .justify_between()
                .items_end()
                .text_size(px(9.))
                .font_family("Consolas")
                .text_color(rgb(if dark { 0xa4abb3 } else { muted }))
                .children(
                    ["0", "−12", "−24", "−36", "−48", "−60"]
                        .into_iter()
                        .map(|label| div().whitespace_nowrap().child(label)),
                ),
        )
        .child(
            div()
                .relative()
                .w(px(28.))
                .h(px(FADER_HEIGHT))
                .child(
                    div()
                        .absolute()
                        .left(px(12.))
                        .top_0()
                        .h_full()
                        .w(px(4.))
                        .rounded(px(2.))
                        .bg(rgb(rail)),
                )
                .when(dark, |track| {
                    track.child(
                        div()
                            .absolute()
                            .left(px(16.))
                            .top_0()
                            .h_full()
                            .w(px(1.))
                            .bg(rgb(rail_highlight)),
                    )
                })
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .top(px(cap_offset))
                        .w(px(28.))
                        .h(px(CAP_HEIGHT))
                        .rounded(px(2.))
                        .border_1()
                        .border_color(rgb(if dark { 0xe4e6e9 } else { cap_border }))
                        .bg(linear_gradient(
                            180.,
                            linear_color_stop(rgb(cap_top), 0.),
                            linear_color_stop(rgb(cap_mid), 1.),
                        ))
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .top(px(5.))
                                .h(px(2.))
                                .bg(rgb(if dark { 0x676d76 } else { cap_bottom })),
                        ),
                ),
        )
        .child(
            div()
                .h(px(FADER_HEIGHT))
                .w(px(11.))
                .flex()
                .items_end()
                .gap(px(3.))
                .child(meter_rail(0, meter, dark, rail))
                .child(meter_rail(1, meter, dark, rail)),
        )
}

fn meter_rail(channel: usize, meter: Option<crate::meter::Display>, dark: bool, rail: u32) -> Div {
    let track = rail;
    let peak_marker = if dark { 0xf2f4f5 } else { 0x243039 };
    let Some(display) = meter else {
        return div()
            .relative()
            .h(px(FADER_HEIGHT))
            .w(px(4.))
            .bg(rgb(track));
    };

    let rms = dbfs_height(display.rms_db[channel]);
    let peak = dbfs_top(display.peak_db[channel]);
    let hold = dbfs_top(display.hold_db[channel]);
    let rms_db = display.rms_db[channel];
    let level_color = if rms_db >= -6.0 {
        if dark { 0xff514a } else { 0xd8292f }
    } else if rms_db >= -18.0 {
        if dark { 0xffc04f } else { 0xc17a00 }
    } else if dark {
        0x61d889
    } else {
        0x168044
    };

    div()
        .relative()
        .h(px(FADER_HEIGHT))
        .w(px(4.))
        .bg(rgb(track))
        .when(display.rms_db[channel] > -60.0, |rail| {
            rail.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .h(px(rms))
                    .bg(rgb(level_color)),
            )
        })
        .when(display.peak_db[channel] > -60.0, |rail| {
            rail.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(peak))
                    .h(px(1.5))
                    .bg(rgb(peak_marker)),
            )
        })
        .when(display.hold_db[channel] > -60.0, |rail| {
            rail.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(hold))
                    .h(px(1.))
                    .bg(rgb(if hold <= FADER_HEIGHT * 0.1 {
                        if dark { 0xff514a } else { 0xd8292f }
                    } else {
                        level_color
                    })),
            )
        })
}

fn dbfs_height(db: f32) -> f32 {
    ((db.clamp(-60.0, 0.0) + 60.0) / 60.0) * FADER_HEIGHT
}

fn dbfs_top(db: f32) -> f32 {
    FADER_HEIGHT - dbfs_height(db)
}

fn mix(foreground: u32, background: u32, percent: u32) -> u32 {
    let channel = |shift: u32| {
        let a = ((foreground >> shift) & 0xffu32) * percent;
        let b = ((background >> shift) & 0xffu32) * (100 - percent);
        (a + b) / 100
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}
