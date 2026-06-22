//! The public [`SimActsPlugin`] — the sim-acts registration unit that wires the
//! `*Requested` message buffers + the per-act dispatch systems (E10.2 / GTW-204; the
//! eighth — reload — added in GTW-275; the ninth — the fieldless [`EndTurnRequested`]
//! turn signal + the [`dispatch_end_turn`] turn-cycle engine — added in GTW-309).

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update, resource_exists};

use crate::{
    acts::{
        downed::{dispatch_execute_downed, dispatch_stabilize_downed},
        fire::{FireDeclaration, dispatch_fire},
        movement::{MoveRejected, MovementOccurred, dispatch_move},
        posture::{dispatch_set_aiming, dispatch_set_facing, dispatch_set_stance},
        reload::{ReloadResult, dispatch_reload},
        request::{
            EndTurnRequested, ExecuteDownedRequested, FireRequested, MoveRequested,
            ReloadRequested, SetAimingRequested, SetFacingRequested, SetStanceRequested,
            StabilizeDownedRequested,
        },
    },
    bleed::{Bleeding, enemy_phase_started, tick_bleed},
    move_acts::{ReactionShotFired, advance_walk},
    occupancy_sync::{SimSystems, sync_destroyed_cover},
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
///
/// **GTW-336 — the §9 bleed-out clock.** The plugin ALSO registers the
/// [`Bleeding`](crate::bleed::Bleeding) signal buffer and adds
/// [`tick_bleed`](crate::bleed::tick_bleed) `.in_set(SimSystems::Simulate)`,
/// `.after(`[`dispatch_end_turn`](crate::turn::dispatch_end_turn)`)` and gated
/// `.run_if(`[`enemy_phase_started`](crate::bleed::enemy_phase_started)`)` — so the
/// bleed-out drain fires once per FULL ROUND, at the enemy-phase start
/// (`docs/combat/resolution.md` §9). Until this slice the drain was unit-test-only; this
/// is what makes Downed gangers actually bleed (and die to the clock) in a live battle and
/// what creates the `Messages<Bleeding>` buffer the presenter's `"Bleeding"` consequence
/// pop is gated on.
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
            // GTW-354: the typed move-REJECTED signal `dispatch_move` emits per commit whose
            // route gate fails — no route (Unreachable) or an unaffordable route
            // (Unaffordable). Presenter-visible like MovementOccurred / ReloadResult; it adds
            // no act logic and no RNG draw, so determinism is preserved.
            .add_message::<MoveRejected>()
            // GTW-336: the §9 bleed-out signal `tick_bleed` emits per Downed ganger that
            // bled this round (the presenter's "Bleeding" FCT pop drains it). Registering
            // the buffer here makes `tick_bleed`'s MessageWriter<Bleeding> param valid and
            // creates the Messages<Bleeding> resource the presenter's consequence reader is
            // gated on (bevy-traps.md #4 / #5).
            .add_message::<Bleeding>()
            // GTW-355: the typed reaction-shot interrupt the committed walk (`advance_walk`)
            // stops on (C5(b)). Registering the buffer here makes `advance_walk`'s
            // MessageReader<ReactionShotFired> param valid. NOTHING in the sim emits it yet —
            // the PRODUCER is GTW-38-future reaction fire / overwatch (the orchestrator will
            // log this); this slice builds the RECEIVING hook only (bevy-traps.md #4 / #5).
            .add_message::<ReactionShotFired>()
            .add_systems(
                Update,
                (
                    dispatch_fire,
                    dispatch_set_aiming,
                    dispatch_set_stance,
                    dispatch_set_facing,
                    dispatch_stabilize_downed,
                    dispatch_execute_downed,
                    dispatch_reload,
                )
                    .in_set(SimSystems::Simulate),
            )
            // GTW-354 (C5): the constrained move dispatch joins the same gated Simulate band
            // but is ordered `.after` the `occupancy_sync` grid-maintenance chain —
            // explicitly `.after(sync_destroyed_cover)`, its LAST system (move → die → cover)
            // — so `find_path` plans over a grid whose occupant slots have already settled
            // this frame (bevy-traps.md #3 — explicit ordering; the recompute_visibility
            // precedent). It writes `Position`/`Tu`; `sync_moved_gangers` reacts to the
            // resulting `Changed<Position>` next frame, so the two compose with no ambiguity.
            .add_systems(
                Update,
                dispatch_move
                    .after(sync_destroyed_cover)
                    .in_set(SimSystems::Simulate),
            )
            // GTW-355 (C6): the committed-walk engine. It advances every WalkInProgress by
            // ONE discrete step per tick — bump-stopping on the LIVE grid, charging each
            // step's planned cost atomically, halting on a reveal or a reaction interrupt.
            // Ordered `.after(dispatch_move)` so a fresh accept's WalkInProgress is visible
            // and its first step lands the same frame (the Commands sync point the ordering
            // forces makes the just-inserted component present this tick), and `.after`
            // the occupancy_sync chain (its bump-stop reads a settled grid). It writes
            // Position/Tu; recompute_visibility is ordered `.after(advance_walk)` (in
            // BattleSimPlugin) so each step's reveal is computed before the NEXT tick reads
            // it (the §44 ambush invariant; bevy-traps.md #3).
            .add_systems(
                Update,
                advance_walk
                    .after(dispatch_move)
                    .after(sync_destroyed_cover)
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
            )
            // GTW-336: wire the §9 bleed-out clock into the live runtime. `tick_bleed`
            // drains a flat tuning BleedRate of Wounds from every un-stabilized Downed
            // ganger ONCE PER FULL ROUND, AT THE ENEMY-PHASE START (resolution.md §9). The
            // turn-cycle engine `dispatch_end_turn` emits a TurnStarted at each advance and
            // the enemy one fires exactly once per full round, so `tick_bleed` runs
            // `.after(dispatch_end_turn)` (so the frame's TurnStarted is buffered) and
            // `.run_if(enemy_phase_started)` (true iff a non-player TurnStarted was emitted
            // this frame — its own independent reader, so it never steals the boundary from
            // the combat-log reader). It joins the BattleInProgress-gated Simulate band, so
            // its Res<CombatTuning> read is panic-free outside a live battle (the band's
            // run_if skips it — bevy-traps.md #1).
            .add_systems(
                Update,
                tick_bleed
                    .after(dispatch_end_turn)
                    .run_if(enemy_phase_started)
                    .in_set(SimSystems::Simulate),
            );
    }
}
