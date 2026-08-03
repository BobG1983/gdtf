//! Battle input plugin registration.

use bevy::prelude::*;
use gdtf_battle_presenter::{GamepadCursorMoved, HighlightRequest, playback_caught_up};
use gdtf_battle_sim::{
    acts::{
        EndTurnRequested, FireRequested, MoveRequested, ReloadRequested, SetAimingRequested,
        SetFacingRequested, SetStanceRequested,
    },
    battle::{PlayerFaction, setup_battle_on_request},
    occupancy_sync::SimSystems,
    prelude::{BattleInProgress, OccupancyGrid},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
};

#[cfg(debug_assertions)]
use super::populate_reg::register_reachable_overlay_population;
use super::{
    populate_reg::{register_fire_target_population, register_path_preview_population},
    surface_reg::{register_contextual_acts, register_gamepad_systems},
};
use crate::{
    InputSystems,
    fire_mode::{SelectedFireMode, sync_fire_mode_on_select},
    gamepad::{ActivePointer, GamepadCursor, gamepad_click_act, gamepad_turn},
    intent::{PendingActIntent, dispatch_act_intents},
    keybinds::{Keybinds, register_keybinds_hot_ron},
    keyboard::{cycle_selection_keys, full_view_key, level_keys, posture_keys, select_clear_key},
    picking::{InspectTarget, emit_highlight_request, pick_hovered_cell},
    selection::{
        PathPreviewTarget, SelectedShooter, auto_select_first_player_ganger,
        clear_downed_selection, left_click_act, right_click_turn_to_face,
        update_selection_highlight,
    },
};

/// Marker resource inserted while the battle input plugin is active.
#[derive(Resource)]
pub struct GdtfBattleInputActive;

/// Registers battle input systems, resources, and messages.
pub struct GdtfBattleInputPlugin;

pub(super) fn battle_act_gate() -> impl SystemCondition<()> {
    resource_exists::<BattleInProgress>
        .and_then(resource_exists::<OccupancyGrid>)
        .and_then(resource_exists::<ButtonInput<MouseButton>>)
        .and_then(resource_exists::<CombatTuning>)
        .and_then(resource_exists::<PlayerFaction>)
        .and_then(resource_exists::<VerticalLinkGraph>)
}

impl Plugin for GdtfBattleInputPlugin {
    #[expect(
        clippy::too_many_lines,
        reason = "one registration list; VIEW vs ACT key splits and register_* helpers already extract the surfaces"
    )]
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            InputSystems::Gather
                .before(SimSystems::Simulate)
                .after(setup_battle_on_request),
        )
        .insert_resource(GdtfBattleInputActive)
        .init_resource::<InspectTarget>()
        .init_resource::<SelectedShooter>()
        .init_resource::<SelectedFireMode>()
        .init_resource::<PathPreviewTarget>()
        .init_resource::<PendingActIntent>()
        .init_resource::<GamepadCursor>()
        .init_resource::<ActivePointer>()
        .add_message::<FireRequested>()
        .add_message::<MoveRequested>()
        .add_message::<SetStanceRequested>()
        .add_message::<SetAimingRequested>()
        .add_message::<SetFacingRequested>()
        .add_message::<ReloadRequested>()
        .add_message::<EndTurnRequested>()
        .add_message::<HighlightRequest>()
        .add_message::<GamepadCursorMoved>()
        .add_systems(
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
                .run_if(
                    resource_exists::<BattleInProgress>.and_then(resource_exists::<PlayerFaction>),
                ),
        )
        .add_systems(
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
        )
        .add_systems(
            Update,
            update_selection_highlight
                .in_set(InputSystems::Gather)
                .after(left_click_act)
                .run_if(
                    resource_exists::<BattleInProgress>.and_then(resource_exists::<OccupancyGrid>),
                ),
        )
        .add_systems(
            Update,
            sync_fire_mode_on_select
                .in_set(InputSystems::Gather)
                .after(left_click_act)
                .run_if(resource_exists::<BattleInProgress>),
        )
        .add_systems(
            Update,
            (level_keys, full_view_key)
                .in_set(InputSystems::Gather)
                .run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<Keybinds>)),
        )
        .add_systems(
            Update,
            (select_clear_key, posture_keys, cycle_selection_keys)
                .in_set(InputSystems::Gather)
                .run_if(
                    resource_exists::<BattleInProgress>
                        .and_then(resource_exists::<Keybinds>)
                        .and_then(playback_caught_up),
                ),
        )
        .add_systems(
            Update,
            dispatch_act_intents
                .in_set(InputSystems::Gather)
                .after(level_keys)
                .after(full_view_key)
                .after(select_clear_key)
                .after(posture_keys)
                .after(cycle_selection_keys)
                .after(left_click_act)
                .after(right_click_turn_to_face)
                .after(gamepad_click_act)
                .after(gamepad_turn)
                .run_if(resource_exists::<BattleInProgress>),
        );

        register_contextual_acts(app);

        register_gamepad_systems(app);

        register_path_preview_population(app);

        #[cfg(debug_assertions)]
        register_reachable_overlay_population(app);

        register_fire_target_population(app);

        register_keybinds_hot_ron(app);
    }
}
