//! Progress: a determinate meter and an indeterminate one.
//!
//! Ported from Ely's `METER_TRACK` (6px) and `ProgressBar`. gpui 0.2.2 has no
//! progress widget, so a meter is a track plus a filled child.

use gpui::{FontWeight, Hsla, ParentElement, SharedString, Styled, div, px, relative};

use crate::ui::theme::{Radius, Theme, TextSize, radius, text_size};

/// The track of a meter, in px.
const TRACK: f32 = 6.0;

/// The width of a filled portion of a meter, as a fraction of the track.
fn fraction(progress: f32) -> f32 {
    progress.clamp(0.0, 1.0)
}

/// A meter showing `progress` in `[0, 1]`.
///
/// The fill is `accent`, not a status color: progress is not good or bad, so it
/// carries no hue of its own.
pub fn progress_bar(progress: f32, theme: Theme) -> impl gpui::IntoElement {
    let colors = &theme.colors;
    div()
        .flex()
        .flex_row()
        .w_full()
        .h(px(TRACK))
        .rounded(px(radius(Radius::Sm)))
        .bg(colors.sunken)
        .overflow_hidden()
        .child(
            div()
                .h_full()
                .rounded(px(radius(Radius::Sm)))
                .bg(colors.accent)
                .w(relative(fraction(progress))),
        )
}

/// A meter with a caption above it: the stage name and a percentage.
///
/// `percent` is None while the total is unknown, which is the one case where a
/// determinate bar would lie about its progress.
pub fn labeled_progress(
    stage: impl Into<SharedString>,
    percent: Option<f32>,
    theme: Theme,
) -> impl gpui::IntoElement {
    let colors = &theme.colors;
    let reading = match percent {
        Some(value) => format!("{}%", (fraction(value) * 100.0).round() as i32),
        None => "Working".to_string(),
    };
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .w_full()
        .child(
            div()
                .flex()
                .flex_row()
                .justify_between()
                .text_size(px(text_size(TextSize::Base)))
                .font_weight(FontWeight(500.0))
                .child(
                    div()
                        .text_color(Hsla::from(colors.fg))
                        .child(stage.into()),
                )
                .child(
                    div()
                        .text_color(Hsla::from(colors.fg_subtle))
                        .font_weight(FontWeight(400.0))
                        .child(reading),
                ),
        )
        .child(progress_bar(percent.unwrap_or(0.0), theme))
}