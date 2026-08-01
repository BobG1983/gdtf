//! [`register_game_commands`] — the ONE walk of [`GAME_COMMANDS`] that wires the host
//! (GTW-942).

use bevy::prelude::*;
use gdtf_battle_input::InputSystems;
use gdtf_qa_command::dispatch::{QaCommandSystems, register_command_set};

use super::set::GAME_COMMANDS;

/// Wire every command in [`GAME_COMMANDS`] and place the command path's two ordered bands
/// inside the game's existing input-gather band.
///
/// The walk itself is `register_command_set`: per command it inits that command's typed
/// queue, its deferral parking, its decode step, the two deadline sweeps, and the
/// command's own handler. Adding a command adds a line to the slice and nothing here.
///
/// The band placement is this host's to choose, and it has to be
/// [`InputSystems::Gather`]: the router that fills the
/// [`CommandInbox`](gdtf_qa_command::dispatch::CommandInbox) is
/// [`route_requests`](crate::dev::net_qa::router::route_requests), which lives there
/// beside every other QA consumer, and a claim outside that band would be unordered
/// against it (`bevy-traps.md` #3). The Route-before-Claim edge itself is NOT chosen here
/// — `register_command_set` declares it, so no host can forget it — this only says WHERE
/// the pair sits.
pub(in crate::dev::net_qa) fn register_game_commands(app: &mut App) {
    app.configure_sets(
        Update,
        (QaCommandSystems::Route, QaCommandSystems::Claim).in_set(InputSystems::Gather),
    );
    register_command_set(app, GAME_COMMANDS);
}
