//! The screens the app can be on.

/// Where the user currently is.
///
/// The library and the sign-in screen are the two live pages; the rest of the
/// navigation surface is added as those pages grow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    /// The sign-in screen.
    Login,
    /// The instance library and its management screens.
    Instances,
}
