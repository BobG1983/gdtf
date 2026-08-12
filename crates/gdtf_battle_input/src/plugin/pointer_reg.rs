use bevy::prelude::*;
use gdtf_battle_presenter::playback_caught_up;
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{BattleInProgress, OccupancyGrid},
};

use super::build::battle_act_gate;
use crate::{
    FireModeSystems, InputSystems,
    fire_mode::sync_fire_mode_on_select,
    intent::dispatch_act_intents,
    picking::{emit_highlight_request, pick_hovered_cell},
    selection::{
        auto_select_first_player_ganger, clear_downed_selection, left_click_act,
        right_click_turn_to_face, update_selection_highlight,
    },
};

pub(super) fn register_hover_and_selection(app: &mut App) {
    app.add_systems(
        Update,
        (
            pick_hovered_cell,
            emit_highlight_request.after(pick_hovered_cell),
        )
            .in_set(InputSystems::Gather)
            .run_if(resource_exists::<BattleInProgress>),
    )
    .add_systems(
        Update,
        clear_downed_selection
            .in_set(InputSystems::Gather)
            .before(auto_select_first_player_ganger)
            .run_if(resource_exists::<BattleInProgress>),
    )
    .add_systems(
        Update,
        auto_select_first_player_ganger
            .in_set(InputSystems::Gather)
            .before(left_click_act)
            .run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<PlayerFaction>)),
    )
    .add_systems(
        Update,
        update_selection_highlight
            .in_set(InputSystems::Gather)
            .after(left_click_act)
            .run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<OccupancyGrid>)),
    )
    .add_systems(
        Update,
        sync_fire_mode_on_select
            .in_set(InputSystems::Gather)
            .in_set(FireModeSystems::Sync)
            .after(left_click_act)
            .run_if(resource_exists::<BattleInProgress>),
    );
}

pub(super) fn register_mouse_clicks(app: &mut App) {
    app.add_systems(
        Update,
        left_click_act
            .in_set(InputSystems::Gather)
            .before(pick_hovered_cell)
            .before(dispatch_act_intents)
            .run_if(battle_act_gate()),
    )
    .add_systems(
        Update,
        right_click_turn_to_face
            .in_set(InputSystems::Gather)
            .before(pick_hovered_cell)
            .before(dispatch_act_intents)
            .run_if(battle_act_gate().and_then(playback_caught_up)),
    );
}
