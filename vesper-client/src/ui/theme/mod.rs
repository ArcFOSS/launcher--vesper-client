//! Vesper's design tokens, ported from Ely.
//!
//! The rule this module exists to enforce: components read semantic tokens off
//! `Theme`, never a literal color or a magic number. Swapping the whole app's
//! appearance is then a `Theme` value, not a find-and-replace.
//!
//! Both Ely modes are built up front; the app renders `Mode::Dark` by default
//! and a toggle only has to swap the `Theme` it hands to its views.

// The token set is authored whole: every type size, density step and elevation
// level belongs to the scale whether or not today's one screen happens to use
// it. Pruning them to silence the lint would leave the scale unusable.
#![allow(dead_code)]

pub mod palette;
pub mod tokens;

use tokens::{control_height, control_padding, elevation_shadow};

pub use palette::{Palette, mix};
pub use tokens::{
    ControlSize, Density, Elevation, PROSE_WIDTH, Radius, TextSize, radius, text_size,
};

/// A `0xRRGGBB` literal as an opaque color.
fn hex_rgba(hex: u32) -> gpui::Rgba {
    gpui::Rgba {
        r: ((hex >> 16) & 0xff) as f32 / 255.0,
        g: ((hex >> 8) & 0xff) as f32 / 255.0,
        b: (hex & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

/// Which set of palette tokens is in effect.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    Light,
    #[default]
    Dark,
}

impl Mode {
    /// True for `Mode::Dark`, for shadow and backdrop decisions.
    pub fn is_dark(self) -> bool {
        self == Mode::Dark
    }
}

/// Font weights. Ely's palette is quiet; weight, not hue, carries emphasis.
///
/// gpui 0.2.2 reads only system fonts, so type uses the platform face at these
/// weights and the sizes in [`tokens::text_size`].
pub mod weights {
    use gpui::FontWeight;

    pub const REGULAR: FontWeight = FontWeight(400.0);
    pub const MEDIUM: FontWeight = FontWeight(500.0);
    pub const SEMIBOLD: FontWeight = FontWeight(600.0);
    pub const BOLD: FontWeight = FontWeight(700.0);
    pub const EXTRABOLD: FontWeight = FontWeight(800.0);
}

/// The resolved theme a view renders against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub mode: Mode,
    pub colors: Palette,
    pub density: Density,
}

impl Default for Theme {
    fn default() -> Self {
        Self::new(Mode::default(), false)
    }
}

impl Theme {
    /// The theme that reads as native to the platform the app is running on:
    /// the Windows UI (Fluent) look on Windows, the GTK (Adwaita) look on Linux,
    /// and the macOS appearance on MacOS. Falls back to the app default.
    pub fn native() -> Self {
        // (mode, accent, on-accent) per platform, in `0xRRGGBB` literals.
        // Emphasis comes from the platform's own accent hue: Windows blue,
        // macOS system blue, Linux Adwaita blue — same quiet treatment Ely's
        // palette gives its near-black ink, but in the host's own color.
        let (mode, accent_hex, on_accent_hex) = match std::env::consts::OS {
            "macos" => (Mode::Light, 0x007AFFu32, 0xFFFFFFu32),
            "windows" => (Mode::Light, 0x0078D4u32, 0xFFFFFFu32),
            "linux" => (Mode::Light, 0x3584E4u32, 0xFFFFFFu32),
            _ => (Mode::Dark, 0x0A84FFu32, 0xF3F1F0u32),
        };
        let accent = hex_rgba(accent_hex);
        let on_accent = hex_rgba(on_accent_hex);
        let mut theme = Self::new(mode, false);
        theme.colors.accent = accent;
        theme.colors.accent_hover = mix(accent, on_accent, 0.12);
        theme.colors.on_accent = on_accent;
        theme
    }

    /// A theme for `mode`, with stronger borders when `high_contrast`.
    pub fn new(mode: Mode, high_contrast: bool) -> Self {
        Self {
            mode,
            colors: Palette::new(mode, high_contrast),
            density: Density::default(),
        }
    }

    /// The same colors, packed tighter.
    pub fn with_density(mut self, density: Density) -> Self {
        self.density = density;
        self
    }

    /// The shadow stack for `level`, tinted for this theme's mode.
    pub fn shadow(&self, level: Elevation) -> Vec<gpui::BoxShadow> {
        elevation_shadow(level, self.mode.is_dark(), &self.colors)
    }

    /// A control's height at `size`, in px.
    pub fn control_height(&self, size: ControlSize) -> f32 {
        control_height(size, self.density)
    }

    /// A control's horizontal padding at `size`, in px.
    pub fn control_padding(&self, size: ControlSize) -> f32 {
        control_padding(size, self.density)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_is_the_default_mode() {
        assert_eq!(Mode::default(), Mode::Dark);
        assert_eq!(Theme::default().mode, Mode::Dark);
    }

    #[test]
    fn a_theme_is_a_pure_function_of_its_mode() {
        assert_eq!(Theme::new(Mode::Light, false), Theme::new(Mode::Light, false));
        assert_ne!(Theme::new(Mode::Light, false), Theme::new(Mode::Dark, false));
    }

    #[test]
    fn density_is_carried_on_the_theme() {
        let theme = Theme::default().with_density(Density::Compact);
        assert_eq!(theme.control_height(ControlSize::Lg), 28.0);
    }
}