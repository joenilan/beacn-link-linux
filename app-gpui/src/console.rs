use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::appearance::Appearance;

const STRIP_HEIGHT: f32 = 420.0;
const FADER_HEIGHT: f32 = 174.0;

/// Render one static Console strip. The application well owns the only click
/// handler; the disabled fader and mute decoration cannot trigger selection.
pub fn strip(
    pair: u8,
    names: Vec<String>,
    empty_label: &'static str,
    light: bool,
    accent: u32,
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
    let route = if names.is_empty() {
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
    let application = names.first().cloned().unwrap_or_else(|| empty_label.into());
    let glyph = names
        .first()
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
            .child(fader_block(
                dark,
                [16., 8., 32., 24.][(pair - 1) as usize],
                colors.muted,
                rail,
                rail_highlight,
                cap_top,
                cap_mid,
                cap_bottom,
                colors.line,
            ))
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
                            .w(px(37.))
                            .text_right()
                            .text_size(px(18.))
                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                            .text_color(rgb(if dark { 0xe5e7e8 } else { colors.text }))
                            .child("—"),
                    )
                    .child(
                        div()
                            .text_size(px(9.))
                            .text_color(rgb(if dark { 0xa3abb4 } else { colors.muted }))
                            .child("dB"),
                    ),
            )
            .child(
                div()
                    .h(px(32.))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(rgb(button_border))
                    .bg(rgb(button))
                    .text_size(px(12.))
                    .text_color(rgb(if dark { 0xcdd2d8 } else { colors.muted }))
                    .child(
                        svg()
                            .path("console/speaker.svg")
                            .size(px(13.))
                            .text_color(rgb(if dark { 0xcdd2d8 } else { 0x45545d })),
                    )
                    .child("Mute"),
            )
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
                        .h(px(13.))
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
                .child(div().h_full().w(px(4.)).bg(rgb(rail)))
                .child(div().h_full().w(px(4.)).bg(rgb(rail))),
        )
}

fn mix(foreground: u32, background: u32, percent: u32) -> u32 {
    let channel = |shift: u32| {
        let a = ((foreground >> shift) & 0xffu32) * percent;
        let b = ((background >> shift) & 0xffu32) * (100 - percent);
        (a + b) / 100
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}
