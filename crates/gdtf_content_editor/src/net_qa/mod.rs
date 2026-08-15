//! Editor net-QA host wiring (debug builds only).
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
pub use commands::{assert_editor_command_set_is_conformant, editor_command_names};
pub use config::EDITOR_QA_SERVER_NAME;
pub use plugin::NetQaEditorPlugin;
pub use schedule::EditorNetQaSystems;
