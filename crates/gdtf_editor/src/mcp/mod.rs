//! Editor MCP host wiring, compiled under the `mcp` feature.
mod assets_root;
mod commands;
mod config;
mod facts;
mod forms;
mod plugin;
mod router;
mod schedule;
mod wire;

pub use assets_root::EditorQaAssetsRoot;
pub use commands::{
    assert_editor_command_set_is_conformant, editor_command_names, shorten_editor_wait_budget,
};
pub use config::EDITOR_QA_SERVER_NAME;
pub use plugin::McpEditorPlugin;
pub use schedule::EditorMcpSystems;
