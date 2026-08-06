//! The game's typed QA commands and the set they are registered from.
pub(crate) mod act;
pub(crate) mod capture;
#[cfg(feature = "headless_test")]
pub(crate) mod conformance;
pub(crate) mod input;
pub(crate) mod lifecycle;
pub(crate) mod procgen;
pub(crate) mod read;
pub(crate) mod register;
pub(crate) mod set;
pub(crate) mod wait;

#[cfg(feature = "headless_test")]
pub use act::{ActCommandSystems, ContextualReply};
#[cfg(feature = "headless_test")]
pub use conformance::{assert_game_command_set_is_conformant, game_command_names};
pub(in crate::dev::net_qa) use register::register_game_commands;
pub(in crate::dev::net_qa) use set::{GAME_COMMANDS, game_host_name};
#[cfg(feature = "headless_test")]
pub use wait::shorten_wait_budget;
