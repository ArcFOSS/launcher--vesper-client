//! The navigation sidebar.

use gpui::{ClickEvent, ElementId, IntoElement, SharedString, Window};
use crate::ui::components::{Button, ButtonVariant};
use crate::ui::theme::{ControlSize, Theme};

/// One sidebar entry: a full-width button that reads as a row.
///
/// The caller wires the press through a GPUI listener so the event lands on the
/// owning view, which owns the page state.
pub fn nav_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    theme: Theme,
    variant: ButtonVariant,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    Button::new(variant)
        .size(ControlSize::Md)
        .full_width()
        .on_click(id, label, theme, on_click)
}
