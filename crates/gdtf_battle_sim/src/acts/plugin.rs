//! The public [`SimActsPlugin`] — the sim-acts registration unit that wires the seven
//! `*Requested` message buffers + the seven per-act dispatch systems (E10.2 / GTW-204).

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update};

use crate::{
    acts::{
        downed::{dispatch_execute_downed, dispatch_stabilize_downed},
        fire::dispatch_fire,
        movement::dispatch_move,
        posture::{dispatch_set_aiming, dispatch_set_facing, dispatch_set_stance},
        request::{
            ExecuteDownedRequested, FireRequested, MoveRequested, SetAimingRequested,
            SetFacingRequested, SetStanceRequested, StabilizeDownedRequested,
        },
    },
    occupancy_sync::SimSystems,
};

/// The **sim-acts registration unit** — registers the seven `*Requested` message buffers
/// and adds the seven per-act dispatch systems, every one `.in_set(SimSystems::Simulate)`
/// in [`Update`] (E10.2 / GTW-204; the seventh — [`MoveRequested`] / [`dispatch_move`] —
/// added in GTW-234).
///
/// This slice CREATES this plugin — E10.0 lands only the [`SimSystems::Simulate`]
/// [`SystemSet`](bevy::prelude::SystemSet) enum and its `configure_sets`; it builds no
/// plugin. In `build()` the plugin:
///
/// - [`add_message`](App::add_message)s [`FireRequested`], [`SetAimingRequested`],
///   [`SetStanceRequested`], [`SetFacingRequested`], [`StabilizeDownedRequested`],
///   [`ExecuteDownedRequested`], and [`MoveRequested`] — exactly once each
///   (`bevy-traps.md` #5; an unregistered message buffer fails a
///   [`MessageReader`](bevy::prelude::MessageReader)'s param validation, the
///   [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)
///   precedent); and
/// - adds the seven dispatch systems to [`Update`] `.in_set(SimSystems::Simulate)`,
///   composing deterministically with the occupancy-maintenance systems already in that
///   set (`bevy-traps.md` #3).
///
/// It consumes E10.0's [`SimSystems::Simulate`] set (it imports and uses it, never
/// redefines it) and does NOT call `configure_sets` — that is E10.0's job, run by
/// whichever plugin owns the set's configuration (the
/// [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)).
/// Wiring this plugin into the `BattleRunning` lifecycle is E10.6, out of scope here.
#[derive(Debug, Default, Clone, Copy)]
pub struct SimActsPlugin;

impl Plugin for SimActsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FireRequested>()
            .add_message::<SetAimingRequested>()
            .add_message::<SetStanceRequested>()
            .add_message::<SetFacingRequested>()
            .add_message::<StabilizeDownedRequested>()
            .add_message::<ExecuteDownedRequested>()
            .add_message::<MoveRequested>()
            .add_systems(
                Update,
                (
                    dispatch_fire,
                    dispatch_set_aiming,
                    dispatch_set_stance,
                    dispatch_set_facing,
                    dispatch_stabilize_downed,
                    dispatch_execute_downed,
                    dispatch_move,
                )
                    .in_set(SimSystems::Simulate),
            );
    }
}
