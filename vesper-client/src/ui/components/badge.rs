//! Badges: a short status tag, such as a Minecraft version or a loader name.

use gpui::{FontWeight, Hsla, ParentElement, SharedString, Styled, div, px};

use crate::ui::theme::{Radius, Theme, TextSize, radius, text_size};

/// Which meaning a badge carries. A badge always means something; it is never
/// decoration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeTone {
    /// No particular meaning: a quiet, neutral tag.
    #[default]
    Neutral,
    /// Installed and working.
    Success,
    /// Needs attention.
    Warning,
    /// Failed or unavailable.
    Danger,
    /// Informational.
    Info,
}

/// The colors a badge tone paints.
fn tone_colors(badge: BadgeTone, theme: Theme) -> (gpui::Rgba, gpui::Rgba) {
    let colors = &theme.colors;
    match badge {
        BadgeTone::Neutral => (colors.hover, colors.fg_muted),
        BadgeTone::Success => (colors.success_subtle, colors.success),
        BadgeTone::Warning => (colors.warning_subtle, colors.warning),
        BadgeTone::Danger => (colors.danger_subtle, colors.danger),
        BadgeTone::Info => (colors.info_subtle, colors.info),
    }
}

/// A short tag: `text` on a tinted wash.
///
/// The wash and the text are the same hue at two strengths, so a badge reads as
/// one color rather than a filled shape with a label on it.
pub fn badge(text: impl Into<SharedString>, badge: BadgeTone, theme: Theme) -> impl gpui::IntoElement {
    let label = text.into();
    let (bg, fg) = tone_colors(badge, theme);
    div()
        .flex()
        .flex_row()
        .items_center()
        .px(px(8.0))
        .py(px(2.0))
        .rounded(px(radius(Radius::Sm)))
        .bg(bg)
        .text_size(px(text_size(TextSize::Sm)))
        .font_weight(FontWeight(500.0))
        .text_color(Hsla::from(fg))
        .child(label)
}

// Call `badge` with the `BadgeTone` that fits; the five tones are the whole
// vocabulary, so no per-tone wrapper earns its keep.