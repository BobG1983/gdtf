//! Editor net-QA host wiring (debug + `net_qa` feature).
mod config;
mod env;
mod plugin;
mod present;
mod router;
mod schedule;
mod screenshot;

pub use config::EDITOR_QA_SERVER_NAME;
pub use plugin::NetQaEditorPlugin;
pub use schedule::EditorNetQaSystems;
pub use screenshot::{
    EditorQaShotDir, EditorScreenshotPayload, EditorShotPollBudget, EditorShotSettle,
    EditorShotSource,
};
