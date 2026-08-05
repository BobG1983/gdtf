//! The game's typed QA commands and the set they are registered from.
pub(crate) mod capture;
#[cfg(feature = "headless_test")]
pub(crate) mod conformance;
pub(crate) mod read;
pub(crate) mod register;
pub(crate) mod set;

#[cfg(feature = "headless_test")]
pub use conformance::{assert_game_command_set_is_conformant, game_command_names};
pub(in crate::dev::net_qa) use register::register_game_commands;
pub(in crate::dev::net_qa) use set::{GAME_COMMANDS, game_host_name};
