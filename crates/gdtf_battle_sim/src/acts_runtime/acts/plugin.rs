//! The public [`SimActsPlugin`] — the sim-acts registration unit that wires the
//! `*Requested` message buffers + the per-act dispatch systems (E10.2 / GTW-204; the
//! eighth — reload — added in GTW-275; the ninth — the fieldless [`EndTurnRequested`]
//! turn signal + the [`dispatch_end_turn`] turn-cycle engine — added in GTW-309).

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update, resource_exists};

use crate::{
    acts::{
        downed::{dispatch_execute_downed, dispatch_stabilize_downed},
        fire::{FireDeclaration, dispatch_fire},
        injury::{InjuryInflicted, apply_injury},
        movement::{MoveRejected, MovementOccurred, dispatch_move},
        posture::{dispatch_set_aiming, dispatch_set_facing, dispatch_set_stance},
        reload::{ReloadResult, dispatch_reload},
        request::{
            EndTurnRequested, ExecuteDownedRequested, FireRequested, MoveRequested,
            ReloadRequested, SetAimingRequested, SetFacingRequested, SetStanceRequested,
            StabilizeDownedRequested,
        },
    },
    ai::enemy_ai_turn,
    bleed::{Bleeding, enemy_phase_started, tick_bleed},
    move_acts::{ReactionShotFired, advance_walk},
    occupancy::project_path_blocking,
    occupancy_sync::{
        CoverDestroyed, GroundAccrued, SimSystems, SlabDestroyed, sync_destroyed_cover,
    },
    reaction::{reaction_trigger, reset_reactions_used},
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
            // GTW-364: the cover-destroyed signal dispatch_fire emits per round whose hit
            // depleted a piece of cover to zero (the fire→deplete→message bridge). Registering
            // the buffer here makes dispatch_fire's MessageWriter<CoverDestroyed> param valid;
            // it is IDEMPOTENT with OccupancyMaintenancePlugin's own add_message::<CoverDestroyed>
            // (Bevy's add_message no-ops a second registration), so both producer (here) and
            // consumer (sync_destroyed_cover / should_recompute_visibility) plugins can name it.
            .add_message::<CoverDestroyed>()
            // GTW-365: the slab-destroyed signal dispatch_fire emits per round whose hit
            // depleted a slab to zero (the slab mirror of CoverDestroyed). Makes
            // dispatch_fire's MessageWriter<SlabDestroyed> param valid; IDEMPOTENT with
            // OccupancyMaintenancePlugin's own add_message::<SlabDestroyed> (the consumer side
            // — sync_destroyed_slab / should_recompute_visibility), so both name it.
            .add_message::<SlabDestroyed>()
            // GTW-366: the ground-accrued signal dispatch_fire emits per round whose hit
            // struck the ground (the ground-accrual mirror of CoverDestroyed/SlabDestroyed).
            // Makes dispatch_fire's MessageWriter<GroundAccrued> param valid; IDEMPOTENT with
            // OccupancyMaintenancePlugin's own add_message::<GroundAccrued> (the consumer side
            // — sync_accrued_ground), so both producer (here) and consumer name it.
            .add_message::<GroundAccrued>()
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
            // GTW-438: the injury signal `dispatch_fire` emits per round whose wound
            // rolled a named injury (the in-fold `roll_injury` froze it onto the report).
            // Registering the buffer here makes `dispatch_fire`'s
            // MessageWriter<InjuryInflicted> + `apply_injury`'s MessageReader valid and
            // creates the Messages<InjuryInflicted> buffer the presenter's GTW-439 FCT/log
            // reader will drain (bevy-traps.md #4 / #5).
            .add_message::<InjuryInflicted>()
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
                    // GTW-501 C3: `find_path` here reads the tag-derived path-blocking
                    // surface, so it must run AFTER `project_path_blocking` re-syncs it this
                    // frame (a Res<OccupancyGrid> reader vs the projection's ResMut writer
                    // must be ordered explicitly — bevy-traps.md #3).
                    .after(project_path_blocking)
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
                    // GTW-501 C3/D2: the bump-stop reads the tag-derived path-blocking
                    // surface (is_path_blocked) — the SAME source the planner reads — so this
                    // ordering is LOAD-BEARING: it must run after the path-blocking re-sync so
                    // the bump-stop sees this frame's marker changes (a `BlocksPathfinding`
                    // added/removed on an in-progress walk's route is honoured the same tick),
                    // keeping planner and executor in lock-step (bevy-traps.md #3).
                    .after(project_path_blocking)
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
            // GTW-70: the minimal enemy-AI brain. It joins the gated Simulate band and is
            // ordered `.after(dispatch_end_turn)` (so it sees the freshly-handed-off
            // ActiveFaction + the enemy team's regenerated TU this frame) and
            // `.before(dispatch_fire)` / `.before(dispatch_move)` (so the REAL FireRequested
            // / MoveRequested it emits dispatch the SAME frame — and the pre-checked accept
            // guarantee holds against the same-frame world). It carries its OWN
            // `run_if(resource_exists::<ActiveFaction>)` so its read stays panic-free
            // outside a live battle (bevy-traps.md #1), and reads no SquadVisibility (it
            // plans on the immutable OmniscientFog), so it adds no ordering ambiguity with
            // the recompute_visibility writer.
            .add_systems(
                Update,
                enemy_ai_turn
                    .run_if(resource_exists::<ActiveFaction>)
                    .after(dispatch_end_turn)
                    // GTW-501 C3: `reachable_within` here reads the tag-derived path-blocking
                    // surface, so it must run AFTER `project_path_blocking` re-syncs it this
                    // frame (read-after-write ordering on OccupancyGrid — bevy-traps.md #3).
                    .after(project_path_blocking)
                    .before(dispatch_fire)
                    .before(dispatch_move)
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
            )
            // GTW-438: the injury applier — drains the InjuryInflicted buffer and folds
            // each rolled injury into its target's InflictedInjuries (via `gain`, tripping
            // Changed) + syncs the standalone BleedAfflicted component. Ordered
            // `.after(dispatch_fire)` so a SAME-FRAME injury (the fire act emitted it this
            // frame) is applied this tick — the resulting Changed<InflictedInjuries> is
            // then projected by `rederive_stats_on_injury_change`, which is ordered
            // `.after(SimSystems::Simulate)` in BattleSimPlugin (so the ledger gain →
            // projector re-derive settles within the frame, before the next tick's stat
            // reads; bevy-traps.md #3 / #7 — query/Commands/MessageReader, no &mut World).
            // It joins the gated Simulate band (no resource it reads is battle-lifetime —
            // the Query + Commands are always valid — so it needs no extra run_if).
            .add_systems(
                Update,
                apply_injury
                    .after(dispatch_fire)
                    .in_set(SimSystems::Simulate),
            )
            // GTW-468 (C5/C8): the LIVE reaction-fire trigger. When a ganger ACTS in an
            // opposing reactor's LOS (a completed movement STEP via Changed<Position> OR a
            // completed FIRE act via the FireDeclaration buffer), it runs the §8 opposed
            // check and, on success, emits a REAL FireRequested (the interrupt shot,
            // consumed by `dispatch_fire`) + a ReactionShotFired (halting a walking actor,
            // consumed by `advance_walk`) and increments the reactor's per-turn cap.
            //
            // THE C5 CYCLE CONSTRAINT (bevy-traps.md #3): a single system cannot be both
            // `.after(advance_walk)` AND `.before(dispatch_fire)` in one frame — dispatch_fire
            // is EARLY, advance_walk is LATE (`.after(dispatch_move)`). Resolution: the trigger
            // detects the COMPLETED act from the PRIOR tick's settled change-detection /
            // buffered declaration and is ordered `.before(dispatch_fire)` AND
            // `.before(advance_walk)`, so BOTH messages it writes are consumed the SAME tick —
            // a documented one-tick cadence between an act and its interrupt (the AC1 shot-fires
            // + AC2 walk-halts tests are the proof it dispatches coherently). It joins the gated
            // Simulate band (its Res grids/tuning + ResMut<ReactionRng> reads are battle-lifetime
            // — the run_if skips it outside a live battle, bevy-traps.md #1). Param-only, no
            // &mut World (bevy-traps.md #7).
            .add_systems(
                Update,
                reaction_trigger
                    .before(dispatch_fire)
                    .before(advance_walk)
                    .in_set(SimSystems::Simulate),
            )
            // GTW-468 (C6): the turn-boundary cap reset. `dispatch_end_turn` emits a
            // TurnStarted at each advance; this zeroes every watcher's ReactionsUsed so the §8
            // per-turn cap applies AFRESH each turn (generalizing "this enemy turn" to "this
            // turn relative to the watcher" — reset every boundary is the defensible default,
            // documented on the system). Ordered `.after(dispatch_end_turn)` so the boundary's
            // TurnStarted is buffered (its own independent reader, so it never steals the
            // boundary from the combat-log / bleed readers — the `tick_bleed` precedent). It
            // joins the gated Simulate band; its Query + MessageReader are always valid, so it
            // needs no extra run_if. Param-only, no &mut World (bevy-traps.md #7).
            .add_systems(
                Update,
                reset_reactions_used
                    .after(dispatch_end_turn)
                    .in_set(SimSystems::Simulate),
            );
    }
}
