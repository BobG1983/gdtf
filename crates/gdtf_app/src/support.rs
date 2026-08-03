//! Visibility helpers for test-support vs normal builds.

/// Expands to `pub` under `test-support`, otherwise `pub(crate)`.
/// Marker UI/state items are not individually documented.
#[cfg(feature = "test-support")]
macro_rules! support_item {
    ($(#[$meta:meta])* enum $($rest:tt)*) => {
        $(#[$meta])*
        #[allow(
            missing_docs,
            reason = "test-support re-exports UI/state markers; docs live on the real public API"
        )]
        pub enum $($rest)*
    };
    ($(#[$meta:meta])* struct $($rest:tt)*) => {
        $(#[$meta])*
        #[allow(
            missing_docs,
            reason = "test-support re-exports UI/state markers; docs live on the real public API"
        )]
        pub struct $($rest)*
    };
    ($(#[$meta:meta])* const fn $($rest:tt)*) => {
        $(#[$meta])*
        #[allow(
            missing_docs,
            reason = "test-support re-exports UI/state markers; docs live on the real public API"
        )]
        pub const fn $($rest)*
    };
    ($(#[$meta:meta])* const $($rest:tt)*) => {
        $(#[$meta])*
        #[allow(
            missing_docs,
            reason = "test-support re-exports UI/state markers; docs live on the real public API"
        )]
        pub const $($rest)*
    };
    ($(#[$meta:meta])* fn $($rest:tt)*) => {
        $(#[$meta])*
        #[allow(
            missing_docs,
            reason = "test-support re-exports UI/state markers; docs live on the real public API"
        )]
        pub fn $($rest)*
    };
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
