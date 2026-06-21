//! The public [`SimActsPlugin`] — the sim-acts registration unit that wires the
//! `*Requested` message buffers + the per-act dispatch systems (E10.2 / GTW-204; the
//! eighth — reload — added in GTW-275; the ninth — the fieldless [`EndTurnRequested`]
//! turn signal + the [`dispatch_end_turn`] turn-cycle engine — added in GTW-309).

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update, resource_exists};

use crate::{
    acts::{
        downed::{dispatch_execute_downed, dispatch_stabilize_downed},
        fire::{FireDeclaration, dispatch_fire},
        movement::{MovementOccurred, dispatch_move},
        posture::{dispatch_set_aiming, dispatch_set_facing, dispatch_set_stance},
        reload::{ReloadResult, dispatch_reload},
        request::{
            EndTurnRequested, ExecuteDownedRequested, FireRequested, MoveRequested,
            ReloadRequested, SetAimingRequested, SetFacingRequested, SetStanceRequested,
            StabilizeDownedRequested,
        },
    },
    occupancy_sync::SimSystems,
    shot_fired::ShotFired,
    turn::{ActiveFaction, TurnStarted, dispatch_end_turn},
};

/// The **sim-acts registration unit** — registers the eight `*Requested` message buffers
/// and adds the eight per-act dispatch systems, every one `.in_set(SimSystems::Simulate)`
/// in [`Update`] (E10.2 / GTW-204; the seventh — [`MoveRequested`] / [`dispatch_move`] —
/// added in GTW-234; the eighth — [`ReloadRequested`] / [`dispatch_reload`] — in
/// GTW-275).
///
/// This slice CREATES this plugin — E10.0 lands only the [`SimSystems::Simulate`]
/// [`SystemSet`](bevy::prelude::SystemSet) enum and its `configure_sets`; it builds no
/// plugin. In `build()` the plugin:
///
/// - [`add_message`](App::add_message)s the eight `*Requested` input buffers
///   ([`FireRequested`], [`SetAimingRequested`], [`SetStanceRequested`],
///   [`SetFacingRequested`], [`StabilizeDownedRequested`], [`ExecuteDownedRequested`],
///   [`MoveRequested`], [`ReloadRequested`]) PLUS the GTW-290 output buffer
///   [`ShotFired`] (emitted by [`dispatch_fire`]) — exactly once each (`bevy-traps.md`
///   #5; an unregistered message buffer fails a
///   [`MessageReader`](bevy::prelude::MessageReader) /
///   [`MessageWriter`](bevy::prelude::MessageWriter)'s param validation, the
///   [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)
///   precedent); and
/// - adds the eight dispatch systems to [`Update`] `.in_set(SimSystems::Simulate)`,
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
            .add_message::<ReloadRequested>()
            // GTW-309: the fieldless end-turn signal the turn-cycle engine drains.
            .add_message::<EndTurnRequested>()
            // GTW-290: the output fire-trajectory signal dispatch_fire emits per round.
            .add_message::<ShotFired>()
            // GTW-312: the output reload-result signal dispatch_reload emits per resolved
            // reload (the three real outcomes; presenter-visible, like ShotFired).
            .add_message::<ReloadResult>()
            // GTW-328: the three output combat-LOG signals — a fire declaration per
            // proceeding shot (dispatch_fire), a move per real step (dispatch_move), and a
            // turn boundary per ActiveFaction advance (dispatch_end_turn). Presenter-only
            // signals, like ShotFired / ReloadResult; they add no fire-result logic and no
            // RNG draw, so determinism is preserved.
            .add_message::<FireDeclaration>()
            .add_message::<MovementOccurred>()
            .add_message::<TurnStarted>()
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
                    dispatch_reload,
                )
                    .in_set(SimSystems::Simulate),
            )
            // GTW-309: the turn-cycle engine joins the gated Simulate band, but takes the
            // battle-lifetime ActiveFaction resource (co-inserted with BattleInProgress),
            // so it carries its OWN resource_exists::<ActiveFaction> run_if to keep its
            // ResMut<ActiveFaction> read panic-free (bevy-traps.md #1).
            .add_systems(
                Update,
                dispatch_end_turn
                    .run_if(resource_exists::<ActiveFaction>)
                    .in_set(SimSystems::Simulate),
            );
    }
}
