//! GDTF application shell: states, scenes, and optional MCP wire types.

mod support;
pub(crate) use support::{support_item, support_use};

mod app;
/// Application entry type.
pub use app::GdtfApp;

mod dev;

#[cfg(feature = "mcp")]
/// MCP wire types for external clients.
pub use dev::mcp::wire as qa_wire;

mod states;

#[cfg(feature = "headless_test")]
/// Test helpers and re-exports of scene markers.
pub mod test_support;
