//! Top-level battle sim plugin: systems, messages, and schedule sets.

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update, resource_exists};

use crate::{
    act_log::wire_act_log,
    acts::{SimActsPlugin, movement::advance_walk},
    battle::{
        messages::{
            BattleLost, BattleReady, BattleWon, SetupBattleRequested, TeardownBattleRequested,
        },
        outcome::check_outcome,
        resources::BattleInProgress,
        setup::setup_battle_on_request,
        teardown::teardown_battle_on_request,
    },
    emplacement::EmplacementTogglePlugin,
    falls::FallsPlugin,
    ganger::{rederive_stats_on_injury_change, rederive_stats_on_tuning_change},
    occupancy::project_vision_blocking,
    occupancy_sync::{OccupancyMaintenancePlugin, SimSystems, sync_destroyed_piece},
    openable::OpenableTogglePlugin,
    peek_sync::{peek_population_needed, sync_peek_offsets},
    visibility::{SquadVisibility, recompute_visibility, should_recompute_visibility},
};

/// Registers battle lifecycle, act systems, and occupancy maintenance.
#[derive(Debug, Default, Clone, Copy)]
pub struct BattleSimPlugin;

impl Plugin for BattleSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(OccupancyMaintenancePlugin)
            .add_plugins(SimActsPlugin);
        wire_act_log(app);
        app.add_plugins(OpenableTogglePlugin)
            .add_plugins(EmplacementTogglePlugin)
            .add_plugins(FallsPlugin)
            .add_message::<SetupBattleRequested>()
            .add_message::<TeardownBattleRequested>()
            .add_message::<BattleReady>()
            .add_message::<BattleWon>()
            .add_message::<BattleLost>()
            .configure_sets(
                Update,
                SimSystems::Simulate.run_if(resource_exists::<BattleInProgress>),
            )
            .configure_sets(
                Update,
                SimSystems::Record.run_if(resource_exists::<BattleInProgress>),
            )
            .add_systems(Update, check_outcome.in_set(SimSystems::Simulate))
            .add_systems(
                Update,
                sync_peek_offsets
                    .in_set(SimSystems::Simulate)
                    .after(advance_walk)
                    .after(sync_destroyed_piece)
                    .before(recompute_visibility)
                    .run_if(peek_population_needed),
            )
            .add_systems(
                Update,
                recompute_visibility
                    .in_set(SimSystems::Simulate)
                    .after(sync_destroyed_piece)
                    .after(advance_walk)
                    .after(project_vision_blocking)
                    .run_if(resource_exists::<SquadVisibility>)
                    .run_if(should_recompute_visibility),
            )
            .add_systems(
                Update,
                (
                    setup_battle_on_request.before(SimSystems::Simulate),
                    teardown_battle_on_request.after(SimSystems::Simulate),
                ),
            )
            .add_systems(
                Update,
                rederive_stats_on_tuning_change.after(setup_battle_on_request),
            )
            .add_systems(
                Update,
                rederive_stats_on_injury_change.after(SimSystems::Simulate),
            );
    }
}
