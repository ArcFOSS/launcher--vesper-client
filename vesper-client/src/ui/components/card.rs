//! Cards: a surface raised off the page, joined by a hairline.

use gpui::{Div, Hsla, ParentElement, Styled, div, px};

use crate::ui::theme::{Elevation, Radius, Theme, radius};

/// A surface at rest, for grouping one thing on a page.
///
/// The shadow is Ely's `Elevation::Raised` for the theme's mode, so a card reads
/// the same in light and dark without a hand-tuned override per mode.
pub struct Card {
    radius: Radius,
    elevated: bool,
}

impl Card {
    /// A flat card: `surface` fill, one hairline, no shadow.
    pub fn new() -> Self {
        Self {
            radius: Radius::Lg,
            elevated: false,
        }
    }

    /// Floats the card off the page.
    pub fn elevated() -> Self {
        Self {
            radius: Radius::Lg,
            elevated: true,
        }
    }

    /// Overrides the corner rounding.
    pub fn radius(mut self, radius: Radius) -> Self {
        self.radius = radius;
        self
    }

    /// The styled card element, with `content` inside it.
    pub fn render(
        self,
        theme: Theme,
        content: impl gpui::IntoElement,
    ) -> impl gpui::IntoElement {
        self.shell(theme).child(content)
    }

    /// The styled card element, still empty, so callers can compose into it.
    pub fn shell(self, theme: Theme) -> Div {
        let colors = &theme.colors;
        let mut element = div()
            .flex()
            .flex_col()
            .rounded(px(radius(self.radius)))
            .bg(colors.surface)
            .border_1()
            .border_color(Hsla::from(colors.border));

        if self.elevated {
            element = element.shadow(theme.shadow(Elevation::Raised));
        }
        element
    }
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}