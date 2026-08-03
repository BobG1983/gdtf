//! `#[cfg(feature = "test-support")] pub(crate) mod test_support` submodule in its
#[cfg(feature = "test-support")]
macro_rules! support_item {
    ($(#[$meta:meta])* enum $($rest:tt)*) => { $(#[$meta])* pub enum $($rest)* };
    ($(#[$meta:meta])* struct $($rest:tt)*) => { $(#[$meta])* pub struct $($rest)* };
    ($(#[$meta:meta])* const fn $($rest:tt)*) => { $(#[$meta])* pub const fn $($rest)* };
    ($(#[$meta:meta])* const $($rest:tt)*) => { $(#[$meta])* pub const $($rest)* };
    ($(#[$meta:meta])* fn $($rest:tt)*) => { $(#[$meta])* pub fn $($rest)* };
}

#[cfg(not(feature = "test-support"))]
macro_rules! support_item {
    ($(#[$meta:meta])* enum $($rest:tt)*) => { $(#[$meta])* pub(crate) enum $($rest)* };
    ($(#[$meta:meta])* struct $($rest:tt)*) => { $(#[$meta])* pub(crate) struct $($rest)* };
    ($(#[$meta:meta])* const fn $($rest:tt)*) => { $(#[$meta])* pub(crate) const fn $($rest)* };
    ($(#[$meta:meta])* const $($rest:tt)*) => { $(#[$meta])* pub(crate) const $($rest)* };
    ($(#[$meta:meta])* fn $($rest:tt)*) => { $(#[$meta])* pub(crate) fn $($rest)* };
}

#[cfg(feature = "test-support")]
macro_rules! support_use {
    ($($rest:tt)*) => { pub use $($rest)* };
}

#[cfg(not(feature = "test-support"))]
macro_rules! support_use {
    ($($rest:tt)*) => { pub(crate) use $($rest)* };
}

pub(crate) use support_item;
pub(crate) use support_use;
