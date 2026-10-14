//! Ely's semantic palette.
//!
//! Ported from `ely-gpui-component` v0.1.1 (`src/theme/palette.rs`), verbatim hex.
//! Components read these tokens, never literals — color carries meaning, not
//! decoration. Each mode is a warm, quiet set; only the semantic roles change.

use gpui::{Hsla, Rgba};

use super::Mode;

/// A color from a `0xRRGGBB` literal, fully opaque.
fn c(hex: u32) -> Rgba {
    Rgba {
        r: ((hex >> 16) & 0xff) as f32 / 255.0,
        g: ((hex >> 8) & 0xff) as f32 / 255.0,
        b: (hex & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

/// A color from a `0xRRGGBB` literal at an explicit alpha.
fn c_alpha(hex: u32, a: f32) -> Rgba {
    Rgba {
        r: ((hex >> 16) & 0xff) as f32 / 255.0,
        g: ((hex >> 8) & 0xff) as f32 / 255.0,
        b: (hex & 0xff) as f32 / 255.0,
        a,
    }
}

/// `color` at an explicit alpha, clamped to `[0, 1]`.
///
/// gpui 0.2.2's `Hsla` — the type `BoxShadow` requires — has no `opacity`
/// method, so shadow tones are derived here rather than at each call site.
/// Scales the existing alpha, so a translucent base stays translucent.
pub fn with_alpha_hsla(color: Hsla, alpha: f32) -> Hsla {
    Hsla {
        a: (color.a * alpha).clamp(0.0, 1.0),
        ..color
    }
}

/// Linear interpolation from `from` to `to` by `t`, clamped to `[0, 1]`.
///
/// Used to derive pressed and hover tones from a base token, so a variant never
/// hardcodes a second color.
pub fn mix(from: Rgba, to: Rgba, t: f32) -> Rgba {
    if t <= 0.0 {
        return from;
    }
    if t >= 1.0 {
        return to;
    }
    let lerp = |x: f32, y: f32| x + (y - x) * t;
    Rgba {
        r: lerp(from.r, to.r),
        g: lerp(from.g, to.g),
        b: lerp(from.b, to.b),
        a: lerp(from.a, to.a),
    }
}

/// Every semantic color. One instance per mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    // Surfaces, from furthest back to nearest front.
    pub bg: Rgba,
    pub surface: Rgba,
    pub sunken: Rgba,
    pub overlay: Rgba,

    // Interaction washes.
    pub hover: Rgba,
    pub active: Rgba,

    // Hairlines.
    pub border: Rgba,
    pub border_strong: Rgba,

    // Text, from strongest to faintest.
    pub fg: Rgba,
    pub fg_muted: Rgba,
    pub fg_subtle: Rgba,
    pub fg_disabled: Rgba,

    // The accent is near-black in both modes: emphasis without hue.
    pub accent: Rgba,
    pub accent_hover: Rgba,
    pub on_accent: Rgba,
    pub on_media: Rgba,

    pub focus: Rgba,
    pub link: Rgba,
    pub selection: Rgba,

    pub success: Rgba,
    pub warning: Rgba,
    pub danger: Rgba,
    pub info: Rgba,

    pub success_subtle: Rgba,
    pub warning_subtle: Rgba,
    pub danger_subtle: Rgba,
    pub info_subtle: Rgba,

    pub backdrop: Rgba,
    pub media_backdrop: Rgba,
    pub shimmer: Rgba,
    pub glass: Rgba,

    pub shadow: Rgba,
    pub tooltip_bg: Rgba,
    pub tooltip_fg: Rgba,
    pub paper: Rgba,
    pub ink: Rgba,
}

impl Palette {
    /// The palette for `mode`, strengthened when `high_contrast`.
    pub fn new(mode: Mode, high_contrast: bool) -> Self {
        match mode {
            Mode::Light => Self::light(high_contrast),
            Mode::Dark => Self::dark(high_contrast),
        }
    }

    /// The light palette: warm off-white paper, ink-dark text.
    fn light(high_contrast: bool) -> Self {
        let mut palette = Self {
            bg: c(0xfcfaf7),
            surface: c(0xffffff),
            sunken: c(0xf6f4f1),
            overlay: c(0xffffff),
            hover: c(0xf2f0ed),
            active: c(0xe9e7e5),
            border: c(0xe1dfdd),
            border_strong: c(0xcccac8),
            fg: c(0x181613),
            fg_muted: c(0x605d5a),
            fg_subtle: c(0x696764),
            fg_disabled: c(0xacaaa8),
            accent: c(0x181613),
            accent_hover: c(0x302d2b),
            on_accent: c(0xfcfaf7),
            on_media: c(0xffffff),
            focus: c(0x3772bb),
            link: c(0x2863ab),
            selection: c_alpha(0x3772bb, 0.2),
            success: c(0x267b4c),
            warning: c(0xa25f12),
            danger: c(0xba3e38),
            info: c(0x2e69b2),
            success_subtle: c(0xe7f9ec),
            warning_subtle: c(0xfef2dd),
            danger_subtle: c(0xffefec),
            info_subtle: c(0xebf4ff),
            backdrop: c_alpha(0x181613, 0.32),
            media_backdrop: c_alpha(0x0e0d0b, 0.98),
            shimmer: c_alpha(0xffffff, 0.7),
            glass: c_alpha(0xfcfaf7, 0.72),
            shadow: c(0x181613),
            tooltip_bg: c(0x181613),
            tooltip_fg: c(0xfcfaf7),
            paper: c(0xffffff),
            ink: c(0x181613),
        };
        if high_contrast {
            palette.fg_muted = c(0x44423f);
            palette.fg_subtle = c(0x575552);
            palette.border = c(0xa6a4a2);
            palette.border_strong = c(0x82807d);
            palette.glass = palette.bg;
        }
        palette
    }

    /// The dark palette: warm near-black, off-white text.
    fn dark(high_contrast: bool) -> Self {
        let mut palette = Self {
            bg: c(0x131110),
            surface: c(0x1b1917),
            sunken: c(0x0e0d0b),
            overlay: c(0x201f1d),
            hover: c(0x252422),
            active: c(0x2c2b29),
            border: c(0x2c2b29),
            border_strong: c(0x413f3d),
            fg: c(0xf3f1f0),
            fg_muted: c(0xafadab),
            fg_subtle: c(0x94928f),
            fg_disabled: c(0x5c5a58),
            accent: c(0xf3f1f0),
            accent_hover: c(0xd9d7d5),
            on_accent: c(0x131110),
            on_media: c(0xffffff),
            focus: c(0x69a1e8),
            link: c(0x7eb1f3),
            selection: c_alpha(0x69a1e8, 0.3),
            success: c(0x6bbc89),
            warning: c(0xe8b45e),
            danger: c(0xe5756e),
            info: c(0x77abec),
            success_subtle: c(0x182c1f),
            warning_subtle: c(0x342611),
            danger_subtle: c(0x3a1d1b),
            info_subtle: c(0x19273a),
            backdrop: c_alpha(0x000000, 0.5),
            media_backdrop: c_alpha(0x000000, 0.98),
            shimmer: c_alpha(0xffffff, 0.06),
            glass: c_alpha(0x131110, 0.72),
            shadow: c(0x000000),
            tooltip_bg: c(0x373533),
            tooltip_fg: c(0xf3f1f0),
            paper: c(0xf3f1f0),
            ink: c(0x181613),
        };
        if high_contrast {
            palette.fg_muted = c(0xcfcdcb);
            palette.fg_subtle = c(0xafadab);
            palette.border = c(0x6a6966);
            palette.border_strong = c(0x858380);
            palette.glass = palette.bg;
        }
        palette
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mode_has_a_stable_palette() {
        assert_eq!(Palette::new(Mode::Light, false), Palette::light(false));
        assert_eq!(Palette::new(Mode::Dark, false), Palette::dark(false));
    }

    #[test]
    fn mixing_hits_both_ends_and_the_middle() {
        let (black, white) = (c(0x000000), c(0xffffff));
        assert_eq!(mix(black, white, 0.0), black);
        assert_eq!(mix(black, white, 1.0), white);
        assert!((mix(black, white, 0.5).g - 0.5).abs() < 1e-4);
    }

    #[test]
    fn alpha_is_clamped() {
        let opaque = Hsla {
            h: 0.0,
            s: 0.0,
            l: 0.0,
            a: 1.0,
        };
        assert_eq!(with_alpha_hsla(opaque, 2.0).a, 1.0);
        assert_eq!(with_alpha_hsla(opaque, -1.0).a, 0.0);
        assert_eq!(with_alpha_hsla(opaque, 0.4).a, 0.4);
    }

    #[test]
    fn high_contrast_strengthens_borders() {
        let (base, hc) = (Palette::light(false), Palette::light(true));
        assert!(hc.border.r < base.border.r);
        let (base, hc) = (Palette::dark(false), Palette::dark(true));
        assert!(hc.border.r > base.border.r);
    }

    #[test]
    fn a_literal_reads_back_channel_for_channel() {
        assert_eq!(c(0x181613).r, 0x18 as f32 / 255.0);
        assert_eq!(c_alpha(0x3772bb, 0.2).a, 0.2);
    }
}