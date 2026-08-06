use bevy::prelude::*;
use gdtf_battle_input::{InputSystems, auto_select_first_player_ganger, dispatch_act_intents};
use gdtf_battle_sim::occupancy_sync::SimSystems;
use gdtf_qa_command::dispatch::{QaCommandSystems, register_command_set};

use super::{act::ActCommandSystems, set::GAME_COMMANDS, wait::probe::count_turn_changes};

pub(in crate::dev::net_qa) fn register_game_commands(app: &mut App) {
    app.configure_sets(
        Update,
        (QaCommandSystems::Route, QaCommandSystems::Claim).in_set(InputSystems::Gather),
    );
    // Every command reads the selection the game has settled for the frame, never the gap
    // before auto-select fills it.
    app.configure_sets(
        Update,
        QaCommandSystems::Claim.after(auto_select_first_player_ganger),
    );
    app.configure_sets(
        Update,
        ActCommandSystems::Claim
            .after(QaCommandSystems::Claim)
            .after(count_turn_changes)
            .before(dispatch_act_intents),
    );
    app.configure_sets(Update, ActCommandSystems::Settle.after(SimSystems::Record));
    register_command_set(app, GAME_COMMANDS);
}
