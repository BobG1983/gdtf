//! Bevy app for GDTF.

mod support;
pub(crate) use support::{support_item, support_use};

mod app;
pub use app::GdtfApp;

/// Whether the DEV-ONLY screenshot / visual-QA capture affordance
/// (`crate::app::capture`, GTW-297) is compiled into THIS build of the crate —
/// true exactly under `cfg!(all(debug_assertions, feature = "dev_capture"))`,
/// the double gate its wiring site uses.
///
/// Exists so the binary crate can PIN the GTW-590 feature fold
/// (`grimdark_turfwar/dynamic_linking` -> `gdtf_app/dev_capture`) with a test
/// that goes red if the fold is ever dropped: before the fold, the pinned QA
/// capture invocation compiled the affordance OUT and silently ignored every
/// `GDTF_CAPTURE_*` env var — a failure invisible to the whole suite.
#[must_use]
pub const fn dev_capture_compiled() -> bool {
    cfg!(all(debug_assertions, feature = "dev_capture"))
}

mod states;

#[cfg(feature = "test-support")]
pub mod test_support;
