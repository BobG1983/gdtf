use bevy::prelude::*;
use gdtf_battle_input::InputSystems;
use gdtf_qa_command::dispatch::{QaCommandSystems, register_command_set};

use super::set::GAME_COMMANDS;

pub(in crate::dev::net_qa) fn register_game_commands(app: &mut App) {
    app.configure_sets(
        Update,
        (QaCommandSystems::Route, QaCommandSystems::Claim).in_set(InputSystems::Gather),
    );
    register_command_set(app, GAME_COMMANDS);
}
