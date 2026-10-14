//! Ely's metric tokens: radius, control sizing, type scale, elevation.
//!
//! Ported from `ely-gpui-component` v0.1.1 (`src/theme/tokens.rs`). Every value
//! is a pure function of its enum, so the whole scale stays inspectable and a
//! change to `Density` moves every control together.

use gpui::{BoxShadow, Hsla, Pixels, point, px, rems};

use super::palette::{Palette, with_alpha_hsla};

/// How tightly controls are packed. Shifts every control height at once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Density {
    Compact,
    #[default]
    Standard,
    Comfortable,
}

/// The three control sizes, small to large.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlSize {
    Sm,
    #[default]
    Md,
    Lg,
}

/// Corner rounding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Radius {
    #[default]
    Sm,
    Md,
    Lg,
    Xl,
}

/// The type scale, smallest to display.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextSize {
    Xs,
    Sm,
    #[default]
    Base,
    Md,
    Lg,
    Xl,
    Xxl,
    Display,
}

/// How far a surface floats above the page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Elevation {
    /// A card at rest on the page.
    #[default]
    Raised,
    /// A menu, tooltip, or popover.
    Floating,
    /// A modal dialog.
    Modal,
}

/// A px measure expressed in rems, so it tracks the window's zoom.
pub fn rem(value: f32) -> gpui::Rems {
    rems(value / 16.0)
}

/// A control's height in px at `size`, shifted by `density`.
pub fn control_height(size: ControlSize, density: Density) -> f32 {
    let base = match size {
        ControlSize::Sm => 24.0,
        ControlSize::Md => 28.0,
        ControlSize::Lg => 32.0,
    };
    base + density_delta(density)
}

/// A control's horizontal padding in px at `size`, shifted by `density`.
pub fn control_padding(size: ControlSize, density: Density) -> f32 {
    let base = match size {
        ControlSize::Sm => 8.0,
        ControlSize::Md => 12.0,
        ControlSize::Lg => 16.0,
    };
    base + density_delta(density) / 2.0
}

/// Corner rounding in px at `size`.
pub fn radius(size: Radius) -> f32 {
    match size {
        Radius::Sm => 4.0,
        Radius::Md => 6.0,
        Radius::Lg => 8.0,
        Radius::Xl => 12.0,
    }
}

/// A font size in px at `size`.
pub fn text_size(size: TextSize) -> f32 {
    match size {
        TextSize::Xs => 11.0,
        TextSize::Sm => 12.0,
        TextSize::Base => 13.0,
        TextSize::Md => 14.0,
        TextSize::Lg => 16.0,
        TextSize::Xl => 20.0,
        TextSize::Xxl => 24.0,
        TextSize::Display => 32.0,
    }
}

/// The shadow stack for `level`, as `(offset_y, blur, alpha)` layers.
///
/// Dark surfaces need a stronger, tighter shadow to read against a near-black
/// page; light surfaces need a softer, wider one.
pub fn elevation(level: Elevation, dark: bool) -> Vec<(f32, f32, f32)> {
    match (level, dark) {
        (Elevation::Raised, false) => vec![(1.0, 2.0, 0.06)],
        (Elevation::Floating, false) => vec![(1.0, 2.0, 0.04), (6.0, 16.0, 0.08)],
        (Elevation::Modal, false) => vec![(2.0, 4.0, 0.04), (16.0, 40.0, 0.14)],
        (Elevation::Raised, true) => vec![(1.0, 2.0, 0.4)],
        (Elevation::Floating, true) => vec![(2.0, 4.0, 0.3), (8.0, 24.0, 0.45)],
        (Elevation::Modal, true) => vec![(4.0, 8.0, 0.3), (20.0, 48.0, 0.55)],
    }
}

/// The shadow stack for `level`, tinted with the palette's shadow color.
pub fn elevation_shadow(
    level: Elevation,
    dark: bool,
    colors: &Palette,
) -> Vec<BoxShadow> {
    let shadow: Hsla = colors.shadow.into();
    elevation(level, dark)
        .into_iter()
        .map(|(offset_y, blur, alpha)| BoxShadow {
            color: with_alpha_hsla(shadow, alpha),
            offset: point(px(0.0), px(offset_y)),
            blur_radius: px(blur),
            spread_radius: px(0.0),
        })
        .collect()
}

/// The track of a progress meter, in px.
pub const METER_TRACK: f32 = 6.0;

/// The widest a centered block of prose runs, in px.
pub const PROSE_WIDTH: f32 = 320.0;

/// A card's border width, in px. Ely draws hairlines.
pub const HAIRLINE: Pixels = px(1.0);

fn density_delta(density: Density) -> f32 {
    match density {
        Density::Compact => -4.0,
        Density::Standard => 0.0,
        Density::Comfortable => 4.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density_moves_every_control_together() {
        let heights = [
            control_height(ControlSize::Lg, Density::Compact),
            control_height(ControlSize::Lg, Density::Standard),
            control_height(ControlSize::Lg, Density::Comfortable),
        ];
        assert_eq!(heights, [28.0, 32.0, 36.0]);
    }

    #[test]
    fn sizes_increase_monotonically() {
        assert!(control_height(ControlSize::Sm, Density::Standard) < 28.0);
        assert!(radius(Radius::Sm) < radius(Radius::Xl));
        assert!(text_size(TextSize::Xs) < text_size(TextSize::Display));
    }

    #[test]
    fn a_modal_sits_above_a_raised_surface() {
        let palette = Palette::new(super::super::Mode::Dark, false);
        let raised = elevation_shadow(Elevation::Raised, true, &palette);
        let modal = elevation_shadow(Elevation::Modal, true, &palette);
        assert_eq!(raised.len(), 1);
        assert_eq!(modal.len(), 2);
        assert!(modal[1].blur_radius > raised[0].blur_radius);
    }
}