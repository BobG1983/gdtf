use bevy::prelude::*;
use gdtf_battle_presenter::{FireTargetHighlight, PathPreview};
#[cfg(debug_assertions)]
use gdtf_battle_presenter::{ReachableCells, ReachableOverlayEnabled};
// FloorCostGrid is only used by the debug-gated reachable overlay populate.
#[cfg(debug_assertions)]
use gdtf_battle_sim::floor::FloorCostGrid;
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{BattleInProgress, OccupancyGrid},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::SquadVisibility,
};

#[cfg(debug_assertions)]
use crate::selection::populate_reachable_overlay;
use crate::{
    FireModeSystems, InputSystems,
    fire_mode::sync_fire_mode_on_select,
    picking::pick_hovered_cell,
    selection::{
        auto_select_first_player_ganger, left_click_act, populate_fire_target,
        populate_path_preview, reset_move_target_on_fire_mode_change,
    },
};

pub(super) fn register_path_preview_population(app: &mut App) {
    app.add_systems(
        Update,
        reset_move_target_on_fire_mode_change
            .in_set(InputSystems::Gather)
            .after(left_click_act)
            .after(auto_select_first_player_ganger)
            .after(sync_fire_mode_on_select)
            .before(populate_path_preview)
            .run_if(resource_exists::<BattleInProgress>),
    )
    .add_systems(
        Update,
        populate_path_preview
            .in_set(InputSystems::Gather)
            .after(left_click_act)
            .after(auto_select_first_player_ganger)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<OccupancyGrid>)
                    .and_then(resource_exists::<VerticalLinkGraph>)
                    .and_then(resource_exists::<SquadVisibility>)
                    .and_then(resource_exists::<CombatTuning>)
                    .and_then(resource_exists::<PathPreview>),
            ),
    );
}

pub(super) fn register_fire_target_population(app: &mut App) {
    app.add_systems(
        Update,
        populate_fire_target
            .in_set(InputSystems::Gather)
            .after(left_click_act)
            .after(auto_select_first_player_ganger)
            .after(FireModeSystems::Write)
            .before(pick_hovered_cell)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<OccupancyGrid>)
                    .and_then(resource_exists::<CombatTuning>)
                    .and_then(resource_exists::<PlayerFaction>)
                    .and_then(resource_exists::<FireTargetHighlight>),
            ),
    );
}

/// Register reachable-range overlay populate. Debug builds only.
#[cfg(debug_assertions)]
pub(super) fn register_reachable_overlay_population(app: &mut App) {
    app.add_systems(
        Update,
        populate_reachable_overlay
            .in_set(InputSystems::Gather)
            .after(left_click_act)
            .after(auto_select_first_player_ganger)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<OccupancyGrid>)
                    .and_then(resource_exists::<VerticalLinkGraph>)
                    .and_then(resource_exists::<SquadVisibility>)
                    .and_then(resource_exists::<CombatTuning>)
                    .and_then(resource_exists::<FloorCostGrid>)
                    .and_then(resource_exists::<ReachableCells>)
                    .and_then(reachable_overlay_enabled),
            ),
    );
}

/// Whether the reachable-range debug overlay is enabled.
#[cfg(debug_assertions)]
fn reachable_overlay_enabled(flag: Option<Res<ReachableOverlayEnabled>>) -> bool {
    flag.is_some_and(|flag| **flag)
}
