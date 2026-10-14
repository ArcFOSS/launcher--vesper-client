//! The primitives every screen is built from.
//!
//! Each component reads tokens off a [`Theme`] and takes explicit inputs — no
//! globals, no hidden state. A component returns an element rather than an
//! entity, so it costs nothing to build and the caller decides when to keep it
//! alive.
//!
//! Layers, bottom to top: `theme` supplies tokens, `components` supply shape,
//! and a view composes them into a screen.

// A component library is written against the screens it will serve, not the
// one that exists today: the sign-in screen needs a primary and a danger
// button, the instance list will need outline, ghost and subtle. Keeping the
// whole vocabulary is the point; unused variants are not accidents.
#![allow(dead_code)]

pub mod badge;
pub mod button;
pub mod card;
pub mod empty_state;
pub mod progress;
pub mod spinner;

// `badge` is reached as `components::badge::badge`, not re-exported here: a
// re-export named `badge` would shadow the module of the same name at every
// use site.
pub use button::{Button, ButtonVariant};
pub use card::Card;
pub use empty_state::{caption, error_message};
pub use spinner::Dots;