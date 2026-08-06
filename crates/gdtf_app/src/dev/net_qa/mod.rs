//! Game net QA control channel (`debug_assertions` + `net_qa` feature).

mod commands;
mod config;
mod env;
mod facts;
mod plugin;
mod present;
mod router;
/// Wire types for external QA clients.
pub mod wire;

crate::support_use!(plugin::NetQaPlugin;);

#[cfg(feature = "headless_test")]
pub use commands::{
    ActCommandSystems, ContextualReply, assert_game_command_set_is_conformant, game_command_names,
    shorten_wait_budget,
};
#[cfg(feature = "headless_test")]
pub use config::{
    NET_QA_PROTOCOL_VERSION, SERVER_NAME as NET_QA_SERVER_NAME, hello_facts as net_qa_hello_facts,
};
#[cfg(feature = "headless_test")]
pub use facts::{
    BattleActivity, BattleModel, BattleScreen, GameFacts, GameFactsParam, PlaybackCatchUp,
    PresenterReadiness, StepperActivity,
};
