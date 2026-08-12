//! Battle input plugin registration.

use bevy::prelude::*;
use gdtf_battle_presenter::{GamepadCursorMoved, HighlightRequest};
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
    key_reg::{register_intent_dispatch, register_keyboard_acts},
    pointer_reg::{register_hover_and_selection, register_mouse_clicks},
    populate_reg::{register_fire_target_population, register_path_preview_population},
    surface_reg::{register_contextual_acts, register_gamepad_systems},
};
use crate::{
    FireModeSystems, InputSystems,
    gamepad::{ActivePointer, GamepadCursor},
    intent::PendingActIntent,
    keybinds::register_keybinds_hot_ron,
    picking::InspectTarget,
    selection::{PathPreviewTarget, SelectedShooter},
};

/// Marker resource inserted while the battle input plugin is active.
#[derive(Resource)]
pub struct GdtfBattleInputActive;

/// Registers battle input systems, resources, and messages.
pub struct GdtfBattleInputPlugin;

/// Run condition every left-click act path carries: the battle resources those systems read.
pub fn battle_act_gate() -> impl SystemCondition<()> {
    resource_exists::<BattleInProgress>
        .and_then(resource_exists::<OccupancyGrid>)
        .and_then(resource_exists::<ButtonInput<MouseButton>>)
        .and_then(resource_exists::<CombatTuning>)
        .and_then(resource_exists::<PlayerFaction>)
        .and_then(resource_exists::<VerticalLinkGraph>)
}

impl Plugin for GdtfBattleInputPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            InputSystems::Gather
                .before(SimSystems::Simulate)
                .after(setup_battle_on_request),
        )
        .configure_sets(
            Update,
            (
                FireModeSystems::Panel.in_set(FireModeSystems::Write),
                FireModeSystems::Command
                    .in_set(FireModeSystems::Write)
                    .after(FireModeSystems::Panel),
            ),
        )
        .insert_resource(GdtfBattleInputActive)
        .init_resource::<InspectTarget>()
        .init_resource::<SelectedShooter>()
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
        .add_message::<GamepadCursorMoved>();

        register_hover_and_selection(app);

        register_mouse_clicks(app);

        register_keyboard_acts(app);

        register_intent_dispatch(app);

        register_contextual_acts(app);

        register_gamepad_systems(app);

        register_path_preview_population(app);

        #[cfg(debug_assertions)]
        register_reachable_overlay_population(app);

        register_fire_target_population(app);

        register_keybinds_hot_ron(app);
    }
}
