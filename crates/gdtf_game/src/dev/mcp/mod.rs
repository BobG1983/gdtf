//! Game MCP control channel, compiled under the `mcp` feature.

mod commands;
mod config;
mod env;
mod facts;
mod plugin;
mod router;
/// Wire types for external QA clients.
pub mod wire;

crate::support_use!(plugin::McpPlugin;);

#[cfg(feature = "headless_test")]
pub use commands::{
    ActCommandSystems, ContextualReply, TurnChangeCount, assert_game_command_set_is_conformant,
    count_turn_changes, game_command_names, shorten_wait_budget,
};
#[cfg(feature = "headless_test")]
pub use config::{
    MCP_PROTOCOL_VERSION, SERVER_NAME as MCP_SERVER_NAME, hello_facts as mcp_hello_facts,
};
#[cfg(all(feature = "headless_test", feature = "dev_tools"))]
pub use facts::StepperActivity;
#[cfg(feature = "headless_test")]
pub use facts::{
    BattleActivity, BattleModel, BattleScreen, GameFacts, GameFactsParam, PlaybackCatchUp,
    PresenterReadiness, TurnOwner,
};
