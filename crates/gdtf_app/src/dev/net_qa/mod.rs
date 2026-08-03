//! `cfg(all(debug_assertions, feature = "net_qa"))` (its wiring site in
mod commands;
mod config;
mod env;
mod facts;
mod plugin;
mod present;
mod router;
mod screenshot;
pub mod wire;

crate::support_use!(plugin::NetQaPlugin;);

#[cfg(feature = "test-support")]
pub use commands::{assert_game_command_set_is_conformant, game_command_names};
#[cfg(feature = "test-support")]
pub use config::{
    NET_QA_PROTOCOL_VERSION, SERVER_NAME as NET_QA_SERVER_NAME, hello_facts as net_qa_hello_facts,
};
#[cfg(feature = "test-support")]
pub use screenshot::{QaShotDir, ScreenshotPayload, ShotPollBudget};
