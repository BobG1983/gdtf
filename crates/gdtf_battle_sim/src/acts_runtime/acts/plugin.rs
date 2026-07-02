//! The public [`SimActsPlugin`] — the sim-acts registration unit that wires the
//! `*Requested` message buffers + the per-act dispatch systems (E10.2 / GTW-204; the
//! eighth — reload — added in GTW-275; the ninth — the fieldless [`EndTurnRequested`]
//! turn signal + the [`dispatch_end_turn`] turn-cycle engine — added in GTW-309).

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update, resource_exists};

use crate::{
    acts::{
        downed::{dispatch_execute_downed, dispatch_stabilize_downed},
        enter_emplacement::{dispatch_enter_emplacement, dispatch_exit_emplacement},
        fire::{FireDeclaration, dispatch_fire},
        injury::{InjuryInflicted, apply_injury},
        melee::dispatch_melee,
        movement::{MoveRejected, MovementOccurred, dispatch_move},
        open_door::dispatch_open_door,
        posture::{dispatch_set_aiming, dispatch_set_facing, dispatch_set_stance},
        reload::{ReloadResult, dispatch_reload},
        request::{
            EndTurnRequested, EnterEmplacementRequested, ExecuteDownedRequested,
            ExitEmplacementRequested, FireRequested, MeleeRequested, MeleeResolved, MoveRequested,
            OpenDoorRequested, ReloadRequested, SetAimingRequested, SetFacingRequested,
            SetStanceRequested, ShoveRequested, StabilizeDownedRequested, ThrowGrenadeRequested,
            ThrowResolved,
        },
        shove::dispatch_shove,
        throw_grenade::dispatch_throw_grenade,
    },
    ai::enemy_ai_turn,
    attachments::apply_pending_attachments,
    bleed::{Bleeding, enemy_phase_started, tick_bleed},
    dot::{DotApplied, DotTicked, apply_dot, tick_dot},
    falls::{FallOccurred, apply_falls},
    fields::{FieldTicked, tick_fields},
    move_acts::{ReactionShotFired, advance_walk},
    occupancy::project_path_blocking,
    occupancy_sync::{
        CoverDestroyed, GroundAccrued, SimSystems, SlabDestroyed, sync_destroyed_cover,
    },
    on_death::{OnDeathOccurred, resolve_on_death},
    reaction::{reaction_trigger, reset_reactions_used},
    shot_fired::ShotFired,
    suppression::{
        SuppressionApplied, apply_suppression, reset_suppression, suppression_auto_stance,
    },
    terrain::{emplacement::SetEmplacement, openable::SetOpenable},
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
        register_messages(app);
        wire_systems(app);
        wire_throw(app); // GTW-546: the blind throw-grenade act (line-cap split)
        wire_attachments(app); // GTW-549: the weapon-attachment application system (line-cap split)
    }
}

/// Register every `*Requested` input buffer + the output signal buffers the sim-acts
/// dispatch systems produce (`bevy-traps.md` #4 / #5) — split out of
/// [`SimActsPlugin::build`] so that method stays under clippy's line gate.
///
/// Registers the input `*Requested` buffers, the GTW-507 [`MeleeResolved`] + GTW-525
/// [`ShoveRequested`] / [`FallOccurred`] buffers, and the structural / combat-log / bleed /
/// injury output buffers — each exactly once (Bevy no-ops a duplicate registration, so the
/// buffers shared with [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)
/// / [`FallsPlugin`](crate::falls::FallsPlugin) are safe to name here too).
fn register_messages(app: &mut App) {
    app.add_message::<FireRequested>()
        .add_message::<SetAimingRequested>()
        .add_message::<SetStanceRequested>()
        .add_message::<SetFacingRequested>()
        .add_message::<StabilizeDownedRequested>()
        .add_message::<ExecuteDownedRequested>()
        .add_message::<MoveRequested>()
        .add_message::<ReloadRequested>()
        // GTW-315: the OPEN-DOOR act's input message — drained by dispatch_open_door. The
        // player-only contextual Open-Door button writes it (input seam -> ActIntent::OpenDoor).
        // Registering the buffer here makes dispatch_open_door's MessageReader<OpenDoorRequested>
        // param valid (bevy-traps.md #4 / #5).
        .add_message::<OpenDoorRequested>()
        // GTW-315: dispatch_open_door WRITES SetOpenable (the GTW-503 open mechanism it reuses).
        // Registering the buffer here makes dispatch_open_door's MessageWriter<SetOpenable> param
        // valid even when SimActsPlugin is wired WITHOUT OpenableTogglePlugin (the in-crate acts
        // test harness). It is IDEMPOTENT with OpenableTogglePlugin's own add_message::<SetOpenable>
        // (Bevy no-ops a second registration), so both the producer (here) and the consumer
        // (apply_openable_toggle in OpenableTogglePlugin) name it — the CoverDestroyed / FallOccurred
        // shared-registration precedent (bevy-traps.md #4 / #5).
        .add_message::<SetOpenable>()
        // GTW-543: the ENTER/EXIT-EMPLACEMENT acts' input messages — drained by
        // dispatch_enter_emplacement / dispatch_exit_emplacement. The player-only contextual
        // Enter/Exit buttons write them (input seam). Registering the buffers here makes those
        // dispatchers' MessageReader params valid (bevy-traps.md #4 / #5).
        .add_message::<EnterEmplacementRequested>()
        .add_message::<ExitEmplacementRequested>()
        // GTW-546: the THROW-GRENADE act's input message — drained by dispatch_throw_grenade.
        // The player-only contextual Throw button writes it (input seam -> ActIntent::ThrowGrenade,
        // the SeamApp phase). Registering the buffer here makes dispatch_throw_grenade's
        // MessageReader<ThrowGrenadeRequested> param valid (bevy-traps.md #4 / #5).
        .add_message::<ThrowGrenadeRequested>()
        // GTW-546: dispatch_throw_grenade emits ThrowResolved per resolved throw (the presenter's
        // impact / blast FX keys off it). Registering the buffer here makes its
        // MessageWriter<ThrowResolved> param valid + creates the Messages<ThrowResolved> buffer the
        // presenter's throw-FX reader (SeamApp phase) gates on (the MeleeResolved precedent).
        .add_message::<ThrowResolved>()
        // GTW-543: the enter/exit dispatchers WRITE SetEmplacement (the GTW-543 toggle mechanism
        // they reuse). Registering the buffer here makes their MessageWriter<SetEmplacement> valid
        // even when SimActsPlugin is wired WITHOUT EmplacementTogglePlugin (the in-crate acts test
        // harness). It is IDEMPOTENT with EmplacementTogglePlugin's own add_message::<SetEmplacement>
        // (Bevy no-ops a second registration), so both the producer (here) and the consumer
        // (apply_emplacement_toggle in EmplacementTogglePlugin) name it — the SetOpenable /
        // CoverDestroyed / FallOccurred shared-registration precedent (bevy-traps.md #4 / #5).
        .add_message::<SetEmplacement>()
        // GTW-507: the LIVE melee act's input message (drained by dispatch_melee) + its
        // presenter-facing output signal (the strike-glyph FX keys off it). Registering both
        // buffers here makes dispatch_melee's MessageReader<MeleeRequested> +
        // MessageWriter<MeleeResolved> params valid (bevy-traps.md #4 / #5) and creates the
        // Messages<MeleeResolved> buffer the presenter's read_melee_resolved FX reader is
        // gated on.
        .add_message::<MeleeRequested>()
        .add_message::<MeleeResolved>()
        // GTW-525: the SHOVE act's input message — drained by dispatch_shove. Carries the
        // deliberate act (input seam -> ActIntent::Shove) AND the weapon-tag auto-shove the
        // melee / fire connect hooks write internally. Registering the buffer here makes
        // dispatch_shove's MessageReader<ShoveRequested> param valid (bevy-traps.md #4/#5).
        .add_message::<ShoveRequested>()
        // GTW-525: dispatch_shove ALSO produces FallOccurred (a shove off a ledge is a real
        // fall — the shared GTW-523 fork emits it). Registering the buffer here makes
        // dispatch_shove's MessageWriter<FallOccurred> param valid; it is IDEMPOTENT with
        // FallsPlugin's own add_message::<FallOccurred> (Bevy no-ops a second registration),
        // so both producers (apply_falls in FallsPlugin, dispatch_shove here) name it and
        // the presenter's fall FX reads ONE buffer regardless of which plugin set is present.
        .add_message::<FallOccurred>()
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
        // GTW-544: the DOT-applied boundary signal `dispatch_fire` emits per round whose
        // penetrating hit from a DOT weapon attached a Dot (the in-fold decision froze it
        // onto the report's `dot_applied`). Registering the buffer here makes
        // `dispatch_fire`'s MessageWriter<DotApplied> + `apply_dot`'s MessageReader valid
        // (bevy-traps.md #4 / #5).
        .add_message::<DotApplied>()
        // GTW-544: the per-round DOT-tick signal `tick_dot` emits per afflicted ganger that
        // took a DOT tick this round (the presenter's DOT FCT pop drains it, the Bleeding
        // precedent). Registering the buffer here makes `tick_dot`'s MessageWriter<DotTicked>
        // param valid and creates the Messages<DotTicked> buffer the presenter reads
        // (bevy-traps.md #4 / #5).
        .add_message::<DotTicked>()
        // GTW-545: the per-round area-damage-field-tick signal `tick_fields` emits per occupant
        // that took a field tick this round (the presenter's field FCT pop drains it, the
        // DotTicked precedent). Registering the buffer here makes `tick_fields`'s
        // MessageWriter<FieldTicked> param valid and creates the Messages<FieldTicked> buffer the
        // presenter reads (bevy-traps.md #4 / #5).
        .add_message::<FieldTicked>()
        // GTW-547: the terminal-death signal emitted from EVERY death / cover-destroyed gate
        // (the ranged/melee/falls ganger kills, the bleed/DOT/field clocks, both cover-destroy
        // sites). `resolve_on_death` drains it to fan each source's authored on-death effect.
        // Registering the buffer here makes every producer's MessageWriter<OnDeathOccurred> +
        // `resolve_on_death`'s MessageReader param valid (bevy-traps.md #4 / #5).
        .add_message::<OnDeathOccurred>()
        // GTW-355: the typed reaction-shot interrupt the committed walk (`advance_walk`)
        // stops on (C5(b)). Registering the buffer here makes `advance_walk`'s
        // MessageReader<ReactionShotFired> param valid. NOTHING in the sim emits it yet —
        // the PRODUCER is GTW-38-future reaction fire / overwatch (the orchestrator will
        // log this); this slice builds the RECEIVING hook only (bevy-traps.md #4 / #5).
        .add_message::<ReactionShotFired>()
        // GTW-526: the presenter-facing suppression signal `apply_suppression` emits once
        // per ganger freshly suppressed this tick (an idempotent refresh emits nothing).
        // Registering the buffer here makes `apply_suppression`'s
        // MessageWriter<SuppressionApplied> param valid and creates the
        // Messages<SuppressionApplied> buffer the presenter's suppression FCT reader will
        // drain (bevy-traps.md #4 / #5).
        .add_message::<SuppressionApplied>();
}

/// Wire every sim-acts dispatch system into the [`SimSystems::Simulate`] band with its
/// explicit ordering (`bevy-traps.md` #3) — split out of [`SimActsPlugin::build`] so that
/// method stays under clippy's line gate.
fn wire_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            dispatch_fire,
            dispatch_set_aiming,
            dispatch_set_stance,
            dispatch_set_facing,
            dispatch_stabilize_downed,
            dispatch_execute_downed,
            dispatch_reload,
            // GTW-507/508: the LIVE melee act. It drains MeleeRequested, gates 8-adjacency
            // (+ LOS/alive/opposing faction on the ganger arm), spends the wielded melee
            // weapon's fight-mode TU, and either runs the §7 opposed-Fight → §5 → §6
            // synthesis onto a ganger (GTW-507, REUSING the GTW-506 core + §4/§5/§6 pieces
            // verbatim) or the GTW-508 UNCONTESTED cover-smash onto an adjacent structure —
            // multiplied (mult_max) weapon damage through CoverLedger::deplete_cover, firing
            // the EXISTING CoverDestroyed signal on a lethal smash. It joins the
            // BattleInProgress-gated Simulate band (its Res grids + tuning + the three ResMut
            // RNG streams are battle-lifetime — the band's run_if skips it outside a live
            // battle, bevy-traps.md #1).
            //
            // Ordering (bevy-traps.md #3): GTW-508 promoted its `cover` param to
            // ResMut<CoverLedger> (the cover-smash arm SPENDS it), so it now DOES share
            // mutable state with sibling `dispatch_fire` (which also holds
            // ResMut<CoverLedger> for the shoot-the-cover depletion). It still joins the
            // UNORDERED group because the shared write is order-INDEPENDENT: Bevy
            // auto-serializes two ResMut on the same resource (never a data race), and the
            // only shared op — `deplete_cover`'s saturating HP decrement — commutes (two
            // decrements on the same cell reach the same remaining HP in either order) and
            // its `destroyed` flag is monotonic (set-once), so whichever dispatcher runs
            // first, the ledger and any emitted CoverDestroyed converge to the same state.
            // The presenter's swap_destroyed_cover / read_cover_destroyed react idempotently.
            // No `.before`/`.after` is needed for correctness.
            dispatch_melee,
            // GTW-315: the LIVE open-door act. It drains OpenDoorRequested, re-gates the door
            // (openable + CLOSED + 8-adjacent to the actor) and the actor (exists + affords the
            // OpenDoorTu leaf), spends that TU off the actor, and WRITES a SetOpenable::toggle for
            // the door — REUSING the GTW-503 open mechanism (it never flips OpenState directly).
            // It joins the UNORDERED group: the only state it shares with a sibling is the
            // SetOpenable message buffer it produces, which OpenableTogglePlugin's
            // apply_openable_toggle CONSUMES — but buffered messages persist a frame
            // (bevy-traps.md #4), so apply_openable_toggle reads it next tick regardless of the
            // producer/consumer intra-frame order (the door's one-frame settle, already documented
            // on the GTW-503 toggle). Its Res<CombatTuning> read is battle-lifetime — taken
            // Option<Res> so it fails closed outside a live battle (bevy-traps.md #1). Its actor
            // (&mut Tu) + door (&OpenState) queries touch disjoint entities, so no B0001 conflict.
            // No .before/.after is needed for correctness.
            dispatch_open_door,
            // GTW-543: the LIVE enter/exit-emplacement acts. They drain EnterEmplacementRequested /
            // ExitEmplacementRequested, re-gate (the emplacement is Vacant + 8-adjacent for enter,
            // or its occupant IS the actor for exit) + the actor (exists + affords the
            // Enter/Exit-EmplacementTu leaf), spend that TU off the actor, and WRITE a
            // SetEmplacement::occupy / ::vacate — REUSING the GTW-543 toggle mechanism (they never
            // flip EmplacementState directly). Like dispatch_open_door they join the UNORDERED
            // group: their only shared state is the SetEmplacement message buffer they produce,
            // which EmplacementTogglePlugin's apply_emplacement_toggle CONSUMES next tick (buffered
            // messages persist a frame, bevy-traps.md #4 — the emplacement's one-frame settle).
            // Res<CombatTuning> is Option<Res> (fail-closed, bevy-traps.md #1); their actor
            // (&mut Tu) + emplacement (&EmplacementState) queries are disjoint (no B0001).
            dispatch_enter_emplacement,
            dispatch_exit_emplacement,
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
    // GTW-525: the SHOVE dispatch — drains ShoveRequested (the deliberate act from the
    // input seam AND the weapon-tag auto-shove the connect hooks write) and resolves the
    // one-cell displacement (pure; the fall it may trigger routes through the shared
    // GTW-523 fork). Ordered (bevy-traps.md #3) `.after(dispatch_fire)` AND
    // `.after(dispatch_melee)` so a SAME-FRAME weapon-tag ShoveRequested — written by the
    // connecting ranged shot (dispatch_fire) or melee strike (dispatch_melee) — is in the
    // buffer when this reads it (else a one-frame lag). Its Res grids/tuning/injury +
    // ResMut fall streams are battle-lifetime, so it joins the gated Simulate band (its
    // Option<Res>/Option<ResMut> params fail closed outside a live battle, bevy-traps.md
    // #1). Param-only, no &mut World (bevy-traps.md #7).
    .add_systems(
        Update,
        dispatch_shove
            .after(dispatch_fire)
            .after(dispatch_melee)
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
    )
    // GTW-526 (C2): the SUPPRESSION producer. It reads every FireRequested this tick and
    // marks each OPPOSING ganger within the tuning SuppressionRadius of the shot's target
    // as Suppressed (anchored to the shooter's origin), emitting a SuppressionApplied on a
    // fresh application. Ordered `.after(dispatch_fire)` so ALL of this frame's
    // FireRequested are visible before it reads (bevy-traps.md #3): the input seam, the
    // enemy AI (`.before(dispatch_fire)`), and the reaction trigger
    // (`.before(dispatch_fire)`) all write FireRequested, and — because each MessageReader
    // has its OWN cursor and messages persist the frame — dispatch_fire draining them does
    // NOT hide them from this system. It joins the BattleInProgress-gated Simulate band;
    // its Res<CombatTuning> read is taken Option<Res> so it fails closed outside a live
    // battle (bevy-traps.md #1). Param-only, no &mut World (bevy-traps.md #7).
    .add_systems(
        Update,
        apply_suppression
            .after(dispatch_fire)
            .in_set(SimSystems::Simulate),
    )
    // GTW-526 (C5): the AUTO-STANCE drop. On a fresh Added<Suppressed> it ducks the ganger
    // behind the cover one step toward the suppressor, writing Stance DIRECTLY (no TU).
    // Ordered `.after(apply_suppression)` (bevy-traps.md #3) so the producer's deferred
    // Commands insert of Suppressed is FLUSHED (the explicit ordering forces a sync point)
    // and this frame's Added<Suppressed> is detected the SAME tick the unit is suppressed.
    // Param-only (Query + Res<CoverLedger>) — the ledger is battle-lifetime, so it joins
    // the gated Simulate band (the run_if skips it outside a live battle, bevy-traps.md #1).
    .add_systems(
        Update,
        suppression_auto_stance
            .after(apply_suppression)
            .in_set(SimSystems::Simulate),
    )
    // GTW-526 (C6): the CLEAR cadence. On a TurnStarted it removes Suppressed from every
    // ganger of the now-active faction (faction-scoped, so a unit stays pinned through the
    // opponent's turn and clears at its OWN turn-start — NOT the faction-agnostic
    // reset_reactions_used cadence). Ordered `.after(dispatch_end_turn)` so the boundary's
    // TurnStarted is buffered (its own independent reader, so it never steals the boundary
    // from the combat-log / bleed / cap-reset readers — the reset_reactions_used
    // precedent). It joins the gated Simulate band; its MessageReader + Query + Commands
    // are always valid, so it needs no extra run_if. Param-only, no &mut World
    // (bevy-traps.md #7).
    .add_systems(
        Update,
        reset_suppression
            .after(dispatch_end_turn)
            .in_set(SimSystems::Simulate),
    );
    wire_clocks(app); // GTW-544/547: the DOT+field clocks + on-death resolver (line-cap split)
}

/// Wire the GTW-549 weapon-attachment application system into the [`SimSystems::Simulate`]
/// band — [`apply_pending_attachments`]. Split out of [`wire_systems`] so that function stays
/// under clippy's line-count gate (the [`wire_clocks`] precedent).
///
/// The wielded-weapon scene spawned by `setup_battle` materializes DEFERRED (on the
/// `SpawnScene` schedule), so the resolved attachment effects ride onto the weapon as a
/// [`PendingAttachments`](crate::weapon::PendingAttachments) component (composed into the
/// scene) rather than being `attach_to_weapon`'d inline (no live entity exists at setup). This
/// system runs on a later tick — once the weapon's stat components + the `PendingAttachments`
/// marker have materialized — reads the marker, and applies each effect via the mandated
/// post-spawn [`attach_to_weapon`](crate::weapon::AttachToWeaponExt::attach_to_weapon)
/// `EntityCommand`, then removes the marker (one-shot). Ordered `.before(dispatch_fire)` so a
/// weapon's attachment stat changes are in place before any shot could read them
/// (bevy-traps.md #3; realistically the marker materializes turns before the player fires). It
/// joins the `BattleInProgress`-gated Simulate band; its Query + Commands are always valid, so
/// it needs no extra `run_if`. Param-only, no `&mut World` (bevy-traps.md #7 — the effect
/// closures' `EntityWorldMut` access is the ticket's sanctioned carve-out).
fn wire_attachments(app: &mut App) {
    app.add_systems(
        Update,
        apply_pending_attachments
            .before(dispatch_fire)
            .in_set(SimSystems::Simulate),
    );
}

/// Wire the GTW-546 throw-grenade act into the [`SimSystems::Simulate`] band — the
/// [`dispatch_throw_grenade`] dispatcher. Split out of [`wire_systems`] so that function stays
/// under clippy's line-count gate (the [`wire_clocks`] precedent).
///
/// The dispatcher drains [`ThrowGrenadeRequested`], re-gates a blind lob (an `Arc` weapon with a
/// loaded round + affordable [`ThrowTu`](crate::tuning::ThrowTu); NO line-of-sight / facing gate),
/// marches the deterministic arc + fans the GTW-541 blast at the landing, and emits
/// [`ThrowResolved`]. It joins the same gated Simulate band as the other acts; UNORDERED — its
/// `ResMut<CoverLedger>` / `ResMut<SlabLedger>` writes commute with `dispatch_fire`'s (the
/// shared-ledger rationale documented on `dispatch_fire`), and its thrower (`&mut Tu`) + weapon
/// (`&mut Magazine`) + target queries touch disjoint entities (no B0001 conflict).
fn wire_throw(app: &mut App) {
    app.add_systems(Update, dispatch_throw_grenade.in_set(SimSystems::Simulate));
}

/// Wire the per-round-clock + on-death systems into the [`SimSystems::Simulate`] band — the
/// GTW-544 [`tick_dot`] / GTW-545 [`tick_fields`] clocks + applier and the GTW-547
/// [`resolve_on_death`] resolver. One combined split out of [`wire_systems`] so that function
/// stays under clippy's line-count gate (the `register_messages` / `wire_systems` split
/// precedent).
fn wire_clocks(app: &mut App) {
    wire_dot(app);
    wire_on_death(app);
}

/// Wire the GTW-547 on-death-effect resolver into the [`SimSystems::Simulate`] band — the
/// [`resolve_on_death`] applier. Split out of [`wire_clocks`] so that helper stays focused.
fn wire_on_death(app: &mut App) {
    app.add_systems(
        Update,
        // GTW-547: `resolve_on_death` drains the OnDeathOccurred buffer and fans each dying
        // source's authored on-death effect (Explode fans a GTW-541 AoE blast; LeaveField spawns
        // a GTW-545 field), resolving cascading explosions to a same-frame fixpoint. Ordered
        // (bevy-traps.md #3) `.after` EVERY death producer so THIS frame's deaths are all
        // buffered before it reads: the ranged/melee kills (dispatch_fire / dispatch_melee) + the
        // per-round bleed/DOT/field clocks (tick_bleed / tick_dot / tick_fields) + the falls kill
        // (apply_falls, wired in the sibling FallsPlugin — the ordering is a no-op if that plugin
        // is absent, and messages persist a frame regardless). It joins the BattleInProgress-gated
        // Simulate band; its Res<OccupancyGrid> / ResMut<FieldRegistry> / Res<CoverOnDeathRegistry>
        // reads are sim-`setup_battle`-inserted battle-lifetime resources, so it carries explicit
        // `resource_exists` run-ifs to stay panic-free if the band runs without them. The field
        // CATALOG (FieldDefRegistry) is app/Load-owned (NOT sim-set), so it is taken Option<Res>
        // INSIDE the system rather than gated on (bevy-traps.md #1).
        resolve_on_death
            .after(dispatch_fire)
            .after(dispatch_melee)
            .after(tick_bleed)
            .after(tick_dot)
            .after(tick_fields)
            .after(apply_falls)
            .run_if(resource_exists::<crate::occupancy::OccupancyGrid>)
            .run_if(resource_exists::<crate::fields::FieldRegistry>)
            .run_if(resource_exists::<crate::on_death::CoverOnDeathRegistry>)
            .in_set(SimSystems::Simulate),
    );
}

/// Wire the GTW-544 damage-over-time systems into the [`SimSystems::Simulate`] band — the
/// [`apply_dot`] boundary applier + the per-round [`tick_dot`] clock. Split out of
/// [`wire_systems`] so that function stays under clippy's line-count gate.
fn wire_dot(app: &mut App) {
    app.add_systems(
        Update,
        // GTW-544: the DOT applier — drains the DotApplied buffer and attaches (or REFRESHES,
        // refresh-not-stack) a Dot on each struck ganger. Ordered `.after(dispatch_fire)` so a
        // SAME-FRAME penetrating DOT hit (the fire act emitted it this frame) is applied this
        // tick (the `apply_injury` precedent). No resource it reads is battle-lifetime (the
        // Query + Commands + MessageReader are always valid), so it needs no extra run_if.
        // Param-only, no &mut World (bevy-traps.md #7).
        apply_dot.after(dispatch_fire).in_set(SimSystems::Simulate),
    )
    // GTW-544: wire the DOT clock into the live runtime — the EXACT `tick_bleed` cadence.
    // `tick_dot` drains each afflicted ganger's Hp DIRECTLY by its Dot's per-turn damage (no
    // armor matchup / injury roll / RNG) ONCE PER FULL ROUND, AT THE ENEMY-PHASE START (the
    // SAME `enemy_phase_started` run condition the §9 bleed-out clock uses, so both fire once
    // per full round). Ordered `.after(dispatch_end_turn)` (so the frame's TurnStarted is
    // buffered) and `.run_if(enemy_phase_started)` (its own independent reader, so it never
    // steals the boundary from the combat-log / bleed readers). It joins the
    // BattleInProgress-gated Simulate band, so its Query is exercised only in a live battle
    // (bevy-traps.md #1).
    .add_systems(
        Update,
        tick_dot
            .after(dispatch_end_turn)
            .run_if(enemy_phase_started)
            .in_set(SimSystems::Simulate),
    )
    // GTW-545: wire the area-damage-field clock into the live runtime — the EXACT `tick_dot` /
    // `tick_bleed` cadence. `tick_fields` reads each fielded cell's occupant off the
    // OccupancyGrid and drains its Hp DIRECTLY by the field's per-turn damage (no armor
    // matchup / injury roll / RNG; gated ONLY by whole-source armor-type immunity) ONCE PER
    // FULL ROUND, AT THE ENEMY-PHASE START (the SAME `enemy_phase_started` run condition), then
    // counts every Turns field down + removes the expired ones. Ordered `.after(dispatch_end_turn)`
    // (so the frame's TurnStarted is buffered) and `.run_if(enemy_phase_started)` (its own
    // independent reader, so it never steals the boundary from the combat-log / bleed / DOT
    // readers), AND explicitly `.after(tick_dot)` (bevy-traps.md #3): both drains share the
    // enemy-phase cadence and can empty one occupant's Hp the same frame, so pinning tick_fields
    // after tick_dot makes a two-source lethal frame's Dead-flip + emit order deterministic
    // (the Dead-flip is idempotent, but the order is pinned). It joins the BattleInProgress-gated
    // Simulate band, so its Res<OccupancyGrid> / ResMut<FieldRegistry> reads are exercised only
    // in a live battle (bevy-traps.md #1).
    .add_systems(
        Update,
        tick_fields
            .after(dispatch_end_turn)
            .after(tick_dot)
            .run_if(enemy_phase_started)
            .run_if(resource_exists::<crate::fields::FieldRegistry>)
            .in_set(SimSystems::Simulate),
    );
}
