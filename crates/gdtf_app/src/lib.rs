//! GDTF application shell: states, scenes, and optional net QA wire types.

mod support;
pub(crate) use support::{support_item, support_use};

mod app;
/// Application entry type.
pub use app::GdtfApp;

mod dev;

#[cfg(debug_assertions)]
/// Net QA wire types for external clients.
pub use dev::net_qa::wire as qa_wire;

mod states;

#[cfg(feature = "headless_test")]
/// Test helpers and re-exports of scene markers.
pub mod test_support;
