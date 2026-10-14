//! An animated loading indicator.
//!
//! Ely's ring spinner needs a transform, which gpui 0.2.2 has no API for, so
//! this ports its `DotsLoader` instead: three dots that brighten in sequence.
//! Only color changes, so no layout is recomputed between frames.
//!
//! The animation is driven off `Window::on_next_frame` rather than a timer
//! task. `Context::spawn` in gpui 0.2.2 takes an `AsyncFnOnce` over a
//! higher-ranked `&mut AsyncApp`, which a closure returning a borrowing future
//! cannot satisfy; a frame callback sidesteps that and keeps the phase tied to
//! wall-clock time, so the dots run at the same speed on any display.

use std::time::{Duration, Instant};

use gpui::{
    App, AppContext, Context, Entity, ParentElement, Render, Styled, WeakEntity, Window, div, px,
};

use crate::ui::theme::{Theme, mix};

/// How long one full dot-brighten cycle takes.
const PERIOD: Duration = Duration::from_millis(900);

/// How many dots a `Dots` shows.
const DOTS: usize = 3;

/// The diameter of one dot, in px.
const DOT: f32 = 5.0;

/// The gap between dots, in px.
const GAP: f32 = 4.0;

/// Where in the cycle `elapsed` lands, in `[0, 1)`.
///
/// Pure and time-based, so the dots run at the same speed whatever the frame
/// rate: a slow frame advances the phase further, it does not stall it.
pub fn phase_at(elapsed: Duration) -> f32 {
    (elapsed.as_secs_f32() / PERIOD.as_secs_f32()).rem_euclid(1.0)
}

/// One dot's brightness at `phase`, peaking as the cycle reaches it.
///
/// Each dot owns a third of the cycle: it ramps up, holds, then fades back down.
fn dot_opacity(phase: f32, index: usize) -> f32 {
    let offset = index as f32 / DOTS as f32;
    let distance = (phase - offset).rem_euclid(1.0) * DOTS as f32;
    if distance < 1.0 {
        distance
    } else {
        (2.0 - distance).max(0.0)
    }
}

/// Three dots that brighten in sequence, looping while the view is on screen.
pub struct Dots {
    started_at: Instant,
}

impl Dots {
    /// A running dots indicator.
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|_cx| Self {
            started_at: Instant::now(),
        })
    }

    /// Redraws one frame from now, which re-arms the loop through `render`.
    fn redraw(this: &WeakEntity<Dots>, cx: &mut App) {
        let Some(strong) = this.upgrade() else {
            return;
        };
        strong.update(cx, |_dots, cx| cx.notify());
    }
}

impl Render for Dots {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        // Armed before returning: each frame's `notify` brings us back here.
        let this = cx.weak_entity();
        window.on_next_frame(move |_window, cx| Self::redraw(&this, cx));
        dots(phase_at(self.started_at.elapsed()), Theme::default())
    }
}

/// A row of dots at `phase`, in `theme`. Pure: same input, same output.
pub fn dots(phase: f32, theme: Theme) -> impl gpui::IntoElement {
    let dim = theme.colors.fg_subtle;
    let bright = theme.colors.fg;
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(GAP))
        .children((0..DOTS).map(|index| {
            div()
                .w(px(DOT))
                .h(px(DOT))
                .rounded(px(DOT / 2.0))
                .bg(mix(dim, bright, dot_opacity(phase, index)))
        }))
}

/// A row of dots at rest, for a caller that does not want the animation loop.
pub fn static_dots(theme: Theme) -> impl gpui::IntoElement {
    dots(0.0, theme)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phase_wraps_around_one_second_and_a_bit() {
        assert!((phase_at(Duration::ZERO) - 0.0).abs() < 1e-6);
        assert!((phase_at(PERIOD) - 0.0).abs() < 1e-6);
        assert!((phase_at(PERIOD / 2) - 0.5).abs() < 1e-3);
        for millis in (0..3000).step_by(37) {
            assert!((0.0..1.0).contains(&phase_at(Duration::from_millis(millis))));
        }
    }

    #[test]
    fn brightness_stays_in_range_across_the_cycle() {
        for step in 0..1000 {
            let phase = step as f32 / 1000.0;
            for index in 0..DOTS {
                let value = dot_opacity(phase, index);
                assert!((0.0..=1.0).contains(&value), "{value} at {phase}");
            }
        }
    }

    #[test]
    fn each_dot_peaks_on_its_own_third_of_the_cycle() {
        assert!(dot_opacity(0.0, 0) > dot_opacity(0.0, 1));
        assert!(dot_opacity(1.0 / 3.0, 1) > dot_opacity(1.0 / 3.0, 2));
        // The cycle wraps, so a dot is at its dimmest just before its own turn.
        assert!(dot_opacity(0.999, 1) < dot_opacity(0.4, 1));
    }
}