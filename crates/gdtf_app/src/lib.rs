//! GDTF application shell: states, scenes, and optional net QA wire types.

mod support;
pub(crate) use support::{support_item, support_use};

mod app;
/// Application entry type.
pub use app::GdtfApp;

mod dev;

#[cfg(all(debug_assertions, feature = "net_qa"))]
/// Net QA wire types for external clients.
pub use dev::net_qa::wire as qa_wire;

mod states;

#[cfg(feature = "test-support")]
/// Test helpers and re-exports of scene markers.
pub mod test_support;
