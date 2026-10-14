//! Empty and error states: what a page says when it has nothing to show.

use gpui::{Hsla, ParentElement, SharedString, Styled, div, px};

use crate::ui::theme::{PROSE_WIDTH, Radius, Theme, TextSize, radius, text_size, weights};

/// A quiet explanation, centered, at Ely's prose width.
///
/// `title` says what is missing; `detail` says what to do about it.
pub fn empty_state(
    title: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    theme: Theme,
) -> impl gpui::IntoElement {
    let colors = &theme.colors;
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(8.0))
        .w(px(PROSE_WIDTH))
        .px(px(24.0))
        .py(px(48.0))
        .rounded(px(radius(Radius::Lg)))
        .border_1()
        .border_color(Hsla::from(colors.border))
        .child(
            div()
                .text_size(px(text_size(TextSize::Lg)))
                .font_weight(weights::SEMIBOLD)
                .text_color(Hsla::from(colors.fg))
                .child(title.into()),
        )
        .child(
            div()
                .text_size(px(text_size(TextSize::Base)))
                .text_color(Hsla::from(colors.fg_muted))
                .text_align(gpui::TextAlign::Center)
                .child(detail.into()),
        )
}

/// An inline error, on the palette's danger wash.
///
/// Errors state what happened and stay readable next to whatever failed.
pub fn error_message(message: impl Into<SharedString>, theme: Theme) -> impl gpui::IntoElement {
    let colors = &theme.colors;
    div()
        .flex()
        .flex_row()
        .w_full()
        .px(px(12.0))
        .py(px(10.0))
        .rounded(px(radius(Radius::Md)))
        .bg(colors.danger_subtle)
        .border_1()
        .border_color(Hsla::from(colors.danger))
        .text_size(px(text_size(TextSize::Base)))
        .text_color(Hsla::from(colors.danger))
        .child(message.into())
}

/// A caption under a control: a hint, a count, a unit.
pub fn caption(text: impl Into<SharedString>, theme: Theme) -> impl gpui::IntoElement {
    let colors = &theme.colors;
    div()
        .text_size(px(text_size(TextSize::Sm)))
        .text_color(Hsla::from(colors.fg_subtle))
        .child(text.into())
}