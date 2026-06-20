//! Bevy app for GDTF.

mod support;
pub(crate) use support::{support_item, support_use};

mod app;
pub use app::GdtfApp;

mod states;

#[cfg(feature = "test-support")]
pub mod test_support;
