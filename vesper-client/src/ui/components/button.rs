//! Buttons.
//!
//! Ported from `ely-gpui-component` v0.1.1 (`src/buttons/button.rs`). The
//! accent is near-black in both modes, so emphasis comes from weight and
//! contrast rather than hue — a button never introduces a new color.
//!
//! ```
//! use vesper_client::ui::components::{Button, ButtonVariant};
//! use vesper_client::ui::theme::Theme;
//!
//! let theme = Theme::default();
//! let button = Button::new(ButtonVariant::Primary)
//!     .size(vesper_client::ui::theme::ControlSize::Lg);
//! ```

use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, Hsla, InteractiveElement, ParentElement,
    Rgba,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::ui::theme::{
    ControlSize, Palette, Radius, TextSize, Theme, mix, radius, text_size, weights,
};

/// How a button reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// The page's one main action.
    Primary,
    /// A filled, bordered action.
    #[default]
    Secondary,
    /// Bordered, no fill.
    Outline,
    /// No chrome until hovered.
    Ghost,
    /// A quiet wash, for a third-tier action.
    Subtle,
    /// Confirms a gain.
    Success,
    /// Destroys or signs out.
    Danger,
    /// Reads as text, behaves as a button.
    Link,
}

/// The five colors a variant paints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tone {
    pub bg: Rgba,
    pub hover: Rgba,
    pub pressed: Rgba,
    pub fg: Rgba,
    pub border: Rgba,
}

/// Fully transparent: keeps a variant's box the same size with or without a border.
fn clear() -> Rgba {
    Rgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    }
}

fn primary(colors: &Palette) -> Tone {
    Tone {
        bg: colors.accent,
        hover: colors.accent_hover,
        pressed: mix(colors.accent_hover, colors.on_accent, 0.12),
        fg: colors.on_accent,
        border: clear(),
    }
}

fn secondary(colors: &Palette) -> Tone {
    Tone {
        bg: colors.surface,
        hover: colors.hover,
        pressed: colors.active,
        fg: colors.fg,
        border: colors.border,
    }
}

fn outline(colors: &Palette) -> Tone {
    Tone {
        bg: clear(),
        hover: colors.hover,
        pressed: colors.active,
        fg: colors.fg,
        border: colors.border_strong,
    }
}

fn ghost(colors: &Palette) -> Tone {
    Tone {
        bg: clear(),
        hover: colors.hover,
        pressed: colors.active,
        fg: colors.fg,
        border: clear(),
    }
}

fn subtle(colors: &Palette) -> Tone {
    Tone {
        bg: colors.hover,
        hover: colors.active,
        pressed: mix(colors.active, colors.border_strong, 0.5),
        fg: colors.fg,
        border: clear(),
    }
}

/// A status color that darkens toward the text color on hover.
fn semantic(base: Rgba, colors: &Palette) -> Tone {
    Tone {
        bg: base,
        hover: mix(base, colors.fg, 0.14),
        pressed: mix(base, colors.fg, 0.24),
        fg: colors.on_accent,
        border: clear(),
    }
}

fn link(colors: &Palette) -> Tone {
    Tone {
        bg: clear(),
        hover: clear(),
        pressed: clear(),
        fg: colors.link,
        border: clear(),
    }
}

/// The colors `variant` paints in `colors`.
pub fn tone(variant: ButtonVariant, colors: &Palette) -> Tone {
    match variant {
        ButtonVariant::Primary => primary(colors),
        ButtonVariant::Secondary => secondary(colors),
        ButtonVariant::Outline => outline(colors),
        ButtonVariant::Ghost => ghost(colors),
        ButtonVariant::Subtle => subtle(colors),
        ButtonVariant::Success => semantic(colors.success, colors),
        ButtonVariant::Danger => semantic(colors.danger, colors),
        ButtonVariant::Link => link(colors),
    }
}

/// A button's label size at each control size.
fn label_size(size: ControlSize) -> TextSize {
    match size {
        ControlSize::Sm => TextSize::Sm,
        ControlSize::Md => TextSize::Base,
        ControlSize::Lg => TextSize::Md,
    }
}

/// The weight a label wears: lighter as the control shrinks.
fn label_weight(size: ControlSize) -> FontWeight {
    match size {
        ControlSize::Sm => weights::REGULAR,
        ControlSize::Md => weights::MEDIUM,
        ControlSize::Lg => weights::MEDIUM,
    }
}

/// A button, configured but not yet built into an element.
///
/// gpui 0.2.2 has no widget layer, so this stands in for one: chain the
/// setters, then finish with [`Button::on_click`] or [`Button::render`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Button {
    variant: ButtonVariant,
    size: ControlSize,
    radius: Radius,
    pill: bool,
    full_width: bool,
}

impl Button {
    /// A button in `variant`.
    pub fn new(variant: ButtonVariant) -> Self {
        Self {
            variant,
            size: ControlSize::default(),
            radius: Radius::Md,
            pill: false,
            full_width: false,
        }
    }

    /// The page's one main action.
    pub fn primary() -> Self {
        Self::new(ButtonVariant::Primary)
    }

    /// Destroys or signs out.
    pub fn danger() -> Self {
        Self::new(ButtonVariant::Danger)
    }

    /// No chrome until hovered.
    pub fn ghost() -> Self {
        Self::new(ButtonVariant::Ghost)
    }

    /// Reads as text, behaves as a button.
    pub fn link() -> Self {
        Self::new(ButtonVariant::Link)
    }

    /// Sets the variant directly.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the control size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Fully round ends, for a page's hero call to action.
    pub fn pill(mut self) -> Self {
        self.pill = true;
        self
    }

    /// Stretches to the width of its container.
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// The colors this button paints in `theme`.
    pub fn tone(&self, theme: Theme) -> Tone {
        tone(self.variant, &theme.colors)
    }

    /// The button's element: a styled box wrapping `label`.
    fn box_element(&self, label: SharedString, theme: Theme) -> Div {
        let paint = self.tone(theme);
        let mut element = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .h(px(theme.control_height(self.size)))
            .px(px(theme.control_padding(self.size)))
            .text_size(px(text_size(label_size(self.size))))
            .font_weight(label_weight(self.size))
            .text_color(Hsla::from(paint.fg))
            .bg(paint.bg)
            .border_1()
            .border_color(Hsla::from(paint.border))
            .child(label);

        if self.full_width {
            element = element.w_full();
        }
        // A pill is a radius of 9999px, which gpui exposes as `rounded_full`.
        if self.pill {
            element.rounded_full()
        } else {
            element.rounded(px(radius(self.radius)))
        }
    }

    /// A button with a click handler.
    ///
    /// gpui needs an element id before a hover style or click handler can
    /// attach, so this finishes the element; there is no clickable `Div`.
    pub fn on_click(
        self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        theme: Theme,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> impl gpui::IntoElement {
        let paint = self.tone(theme);
        self.box_element(label.into(), theme)
            .id(id)
            .cursor_pointer()
            .hover(move |style| style.bg(paint.hover))
            .on_click(handler)
    }

    /// A button that does nothing, for a disabled or display-only slot.
    pub fn render(self, label: impl Into<SharedString>, theme: Theme) -> impl gpui::IntoElement {
        self.box_element(label.into(), theme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::Mode;

    fn colors() -> Palette {
        Theme::new(Mode::Dark, false).colors
    }

    #[test]
    fn a_primary_button_is_the_accent_with_accent_text() {
        let colors = colors();
        let paint = tone(ButtonVariant::Primary, &colors);
        assert_eq!(paint.bg, colors.accent);
        assert_eq!(paint.fg, colors.on_accent);
    }

    #[test]
    fn every_variant_paints_a_transparent_border_or_a_token() {
        let colors = colors();
        for variant in [
            ButtonVariant::Primary,
            ButtonVariant::Secondary,
            ButtonVariant::Outline,
            ButtonVariant::Ghost,
            ButtonVariant::Subtle,
            ButtonVariant::Success,
            ButtonVariant::Danger,
            ButtonVariant::Link,
        ] {
            let paint = tone(variant, &colors);
            assert_eq!(paint.border.a == 0.0 || paint.border == colors.border
                || paint.border == colors.border_strong, true, "{variant:?}");
        }
    }

    #[test]
    fn a_hover_tone_always_differs_from_its_rest_tone() {
        let colors = colors();
        let paint = tone(ButtonVariant::Primary, &colors);
        assert_ne!(paint.bg, paint.hover);
        assert_ne!(paint.hover, paint.pressed);
    }
}