//! Bevy app for GDTF.

/// Declares an item with `pub` visibility when the `test-support` feature is
/// enabled, and `pub(crate)` visibility otherwise.
///
/// The state enums and [`scenes::ScenesPlugin`] are crate-internal in normal
/// builds (the production public API is exactly `GdtfApp`). With `test-support`
/// on, an external crate must be able to *name* them through
/// [`test_support`], so their definitions widen to `pub`. Wrapping each
/// definition in this macro keeps the visibility flip in one place and lets
/// `unreachable_pub` stay satisfied in both configurations. It wraps `enum` /
/// `struct` item definitions and free / inherent `fn` items (the GTW-223
/// auto-battle affordance widens its `auto_battle_enabled` gate + its `from_env`
/// constructor this way).
#[cfg(feature = "test-support")]
macro_rules! support_item {
    ($(#[$meta:meta])* enum $($rest:tt)*) => { $(#[$meta])* pub enum $($rest)* };
    ($(#[$meta:meta])* struct $($rest:tt)*) => { $(#[$meta])* pub struct $($rest)* };
    ($(#[$meta:meta])* fn $($rest:tt)*) => { $(#[$meta])* pub fn $($rest)* };
}

/// Declares an item with `pub` visibility when the `test-support` feature is
/// enabled, and `pub(crate)` visibility otherwise. See the `test-support`
/// variant for the rationale.
#[cfg(not(feature = "test-support"))]
macro_rules! support_item {
    ($(#[$meta:meta])* enum $($rest:tt)*) => { $(#[$meta])* pub(crate) enum $($rest)* };
    ($(#[$meta:meta])* struct $($rest:tt)*) => { $(#[$meta])* pub(crate) struct $($rest)* };
    ($(#[$meta:meta])* fn $($rest:tt)*) => { $(#[$meta])* pub(crate) fn $($rest)* };
}

/// Re-exports a path with `pub` visibility when the `test-support` feature is
/// enabled, and `pub(crate)` visibility otherwise.
///
/// The `mod.rs` re-export of a widened item caps the item's visibility, so the
/// re-export chain that [`test_support`] reaches through must widen in lockstep
/// with [`support_item`]; otherwise the public re-export hits E0365.
#[cfg(feature = "test-support")]
macro_rules! support_use {
    ($($rest:tt)*) => { pub use $($rest)* };
}

/// Re-exports a path with `pub` visibility when the `test-support` feature is
/// enabled, and `pub(crate)` visibility otherwise. See the `test-support`
/// variant for the rationale.
#[cfg(not(feature = "test-support"))]
macro_rules! support_use {
    ($($rest:tt)*) => { pub(crate) use $($rest)* };
}

pub(crate) use support_item;
pub(crate) use support_use;

mod app;
pub use app::GdtfApp;

mod scenes;
mod states;

#[cfg(feature = "test-support")]
pub mod test_support;
