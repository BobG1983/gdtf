use bevy::prelude::*;
use cobalt_mcp_command::dispatch::{McpCommandSystems, register_command_set};
use gdtf_battle_input::{
    InputSystems, auto_select_first_player_ganger, contextual::ContextualActSystems,
    dispatch_act_intents,
};
use gdtf_battle_sim::occupancy_sync::SimSystems;

use super::{act::ActCommandSystems, set::GAME_COMMANDS, wait::probe::count_turn_changes};
use crate::states::running::game::battlescape::contextual_panel::registrar::ContextualPanelSystems;

pub(in crate::dev::mcp) fn register_game_commands(app: &mut App) {
    app.configure_sets(
        Update,
        (McpCommandSystems::Route, McpCommandSystems::Claim).in_set(InputSystems::Gather),
    );
    // Every command reads the selection the game has settled for the frame, never the gap
    // before auto-select fills it.
    app.configure_sets(
        Update,
        McpCommandSystems::Claim.after(auto_select_first_player_ganger),
    );
    app.configure_sets(
        Update,
        ActCommandSystems::Claim
            .after(McpCommandSystems::Claim)
            .after(count_turn_changes)
            .before(dispatch_act_intents),
    );
    app.configure_sets(
        Update,
        ActCommandSystems::Claim.before(ContextualActSystems::Drain),
    );
    // Only the contextual acts read the panel's offer, so only they wait on the scan that
    // writes it.
    app.configure_sets(
        Update,
        ActCommandSystems::ContextualClaim
            .in_set(ActCommandSystems::Claim)
            .after(ContextualPanelSystems::Offer),
    );
    app.configure_sets(Update, ActCommandSystems::Settle.after(SimSystems::Record));
    register_command_set(app, GAME_COMMANDS);
}
