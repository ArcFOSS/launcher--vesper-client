//! Vesper Client's GPUI interface.
//!
//! Three layers, each depending only on the one below it: `theme` holds the
//! design tokens, `components` turns them into shape, and views compose
//! components into screens.

pub mod components;
pub mod login_view;
pub mod theme;