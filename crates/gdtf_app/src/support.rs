//! Visibility helpers for test-support vs normal builds.

/// Expands to `pub` under `test-support`, otherwise `pub(crate)`, whether or not the
/// item spells `pub(crate)` itself. Marker items are not individually documented.
#[cfg(feature = "headless_test")]
macro_rules! support_item {
    ($(#[$meta:meta])* pub(crate) $($rest:tt)*) => {
        $(#[$meta])*
        pub $($rest)*
    };
    ($(#[$meta:meta])* enum $($rest:tt)*) => {
        $(#[$meta])*
        pub enum $($rest)*
    };
    ($(#[$meta:meta])* struct $($rest:tt)*) => {
        $(#[$meta])*
        pub struct $($rest)*
    };
    ($(#[$meta:meta])* const fn $($rest:tt)*) => {
        $(#[$meta])*
        pub const fn $($rest)*
    };
    ($(#[$meta:meta])* const $($rest:tt)*) => {
        $(#[$meta])*
        pub const $($rest)*
    };
    ($(#[$meta:meta])* fn $($rest:tt)*) => {
        $(#[$meta])*
        pub fn $($rest)*
    };
}

#[cfg(not(feature = "headless_test"))]
macro_rules! support_item {
    ($(#[$meta:meta])* pub(crate) $($rest:tt)*) => { $(#[$meta])* pub(crate) $($rest)* };
    ($(#[$meta:meta])* enum $($rest:tt)*) => { $(#[$meta])* pub(crate) enum $($rest)* };
    ($(#[$meta:meta])* struct $($rest:tt)*) => { $(#[$meta])* pub(crate) struct $($rest)* };
    ($(#[$meta:meta])* const fn $($rest:tt)*) => { $(#[$meta])* pub(crate) const fn $($rest)* };
    ($(#[$meta:meta])* const $($rest:tt)*) => { $(#[$meta])* pub(crate) const $($rest)* };
    ($(#[$meta:meta])* fn $($rest:tt)*) => { $(#[$meta])* pub(crate) fn $($rest)* };
}

#[cfg(feature = "headless_test")]
macro_rules! support_use {
    ($($rest:tt)*) => { pub use $($rest)* };
}

#[cfg(not(feature = "headless_test"))]
macro_rules! support_use {
    ($($rest:tt)*) => { pub(crate) use $($rest)* };
}

pub(crate) use support_item;
pub(crate) use support_use;
