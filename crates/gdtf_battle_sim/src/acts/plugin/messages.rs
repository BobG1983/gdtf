//! Message-buffer registration — every `*Requested` input buffer + output signal
//! buffer the sim-acts dispatch systems produce (changes when a new signal is added).

use bevy::prelude::App;

use crate::{
    acts::{
        fire::FireDeclaration,
        injury::InjuryInflicted,
        movement::{MoveRejected, MovementOccurred, ReactionShotFired},
        reload::ReloadResult,
        request::{
            EndTurnRequested, EnterEmplacementRequested, ExecuteDownedRequested,
            ExitEmplacementRequested, FireRequested, MeleeRequested, MeleeResolved, MeleeStruck,
            MoveRequested, OpenDoorRequested, ReloadRequested, SetAimingRequested,
            SetFacingRequested, SetStanceRequested, ShoveRequested, StabilizeDownedRequested,
            ThrowGrenadeRequested, ThrowResolved,
        },
    },
    effects::{
        bleed::{BleedStarted, Bleeding},
        dot::{DotAfflicted, DotApplied, DotTicked},
        fields::{FieldAfflicted, FieldTicked},
        on_death::OnDeathOccurred,
    },
    falls::FallOccurred,
    occupancy_sync::{CoverDestroyed, GroundAccrued, SlabDestroyed},
    reaction::InterruptDeclared,
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
    terrain::{emplacement::SetEmplacement, openable::SetOpenable},
    turn::TurnStarted,
};

/// Register every `*Requested` input buffer + the output signal buffers the sim-acts
/// dispatch systems produce (`bevy-traps.md` #4 / #5) — split out of
/// [`SimActsPlugin::build`](super::SimActsPlugin) so that method stays under clippy's line gate.
///
/// Registers the input `*Requested` buffers, the GTW-507 [`MeleeResolved`] + GTW-525
/// [`ShoveRequested`] / [`FallOccurred`] buffers, and the structural / combat-log / bleed /
/// injury output buffers — each exactly once (Bevy no-ops a duplicate registration, so the
/// buffers shared with [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)
/// / [`FallsPlugin`](crate::falls::FallsPlugin) are safe to name here too).
pub(super) fn register_messages(app: &mut App) {
    app.add_message::<FireRequested>()
        .add_message::<SetAimingRequested>()
        .add_message::<SetStanceRequested>()
        .add_message::<SetFacingRequested>()
        .add_message::<StabilizeDownedRequested>()
        .add_message::<ExecuteDownedRequested>()
        .add_message::<MoveRequested>()
        .add_message::<ReloadRequested>()
        // GTW-315: the OPEN-DOOR act's input message — drained by dispatch_open_door. The
        // player-only contextual Open-Door button writes it (the input seam's per-act queue).
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
        // The player-only contextual Throw button writes it (the input seam's per-act queue).
        // Registering the buffer here makes dispatch_throw_grenade's
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
        // GTW-572: the NUMBER-BEARING melee fact — one per CONNECTING ganger strike, carrying
        // both combatants + the applied HP loss so the combat log can phrase a melee-damage
        // line (MeleeResolved stays the cell+damage-type strike-GLYPH signal). Registering
        // the buffer here makes dispatch_melee's MessageWriter<MeleeStruck> param valid and
        // creates the Messages<MeleeStruck> buffer the app's log forwarder is gated on.
        .add_message::<MeleeStruck>()
        // GTW-525: the SHOVE act's input message — drained by dispatch_shove. Carries the
        // deliberate act (the input seam's contextual Shove press) AND the weapon-tag auto-shove the
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
        // GTW-572: the armor-broken crossing fact — `dispatch_fire`'s ganger arm and
        // `dispatch_melee`'s connecting strike both bridge the §6 wear's
        // protecting→broken crossing into this buffered message (the presenter's spark
        // flash / FCT tag and the combat log's armor-broken line drain it). Registering
        // the buffer here makes both writers' params valid (bevy-traps.md #4/#5).
        .add_message::<crate::armor_wear::ArmorBroken>()
        // GTW-572: the once-per-span bleed affliction-start fact `tick_bleed` emits on the
        // FIRST draining tick of a span (the combat log's bleeding line drains it — the Q2
        // ruling: once at affliction start, never per tick). Registering the buffer here
        // makes `tick_bleed`'s MessageWriter<BleedStarted> param valid (bevy-traps.md #4/#5).
        .add_message::<BleedStarted>()
        // GTW-438: the injury signal `dispatch_fire` emits per round whose wound
        // rolled a named injury (the in-fold `roll_injury` froze it onto the report).
        // Registering the buffer here makes `dispatch_fire`'s
        // MessageWriter<InjuryInflicted> + `apply_injury`'s MessageReader valid and
        // creates the Messages<InjuryInflicted> buffer the presenter's GTW-439 FCT/log
        // reader will drain (bevy-traps.md #4 / #5).
        .add_message::<InjuryInflicted>()
        // GTW-544: the DOT-applied boundary signal `dispatch_fire` emits per round whose
        // penetrating hit from a DOT weapon attached a Dot (the in-fold decision froze it
        // onto the ganger verdict's `dot_applied`). Registering the buffer here makes
        // `dispatch_fire`'s MessageWriter<DotApplied> + `apply_dot`'s MessageReader valid
        // (bevy-traps.md #4 / #5).
        .add_message::<DotApplied>()
        // GTW-572: the once-per-span DOT affliction-start fact `apply_dot` emits ONLY on the
        // fresh-ATTACH branch (a refresh is mid-affliction — the Q2 ruling: once at affliction
        // start, never per tick). Registering the buffer here makes `apply_dot`'s
        // MessageWriter<DotAfflicted> param valid (bevy-traps.md #4/#5).
        .add_message::<DotAfflicted>()
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
        // GTW-572: the once-per-span field exposure-start fact `tick_fields` emits the FIRST
        // round a live field drains an occupant (the combat log's field line drains it — the
        // Q2 ruling: once at affliction start, never per tick). Registering the buffer here
        // makes `tick_fields`'s MessageWriter<FieldAfflicted> param valid (bevy-traps.md #4/#5).
        .add_message::<FieldAfflicted>()
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
        // GTW-727 C5: the reaction-fire EXPOSURE signal — WHICH reactor is interrupting WHOSE
        // act. `reaction_trigger` writes it (through its InterruptSignals bundle), and this
        // plugin is what registers `reaction_trigger`, so this is the producer registering its
        // own buffer: without it EVERY harness that wires SimActsPlugin fails the writer's
        // param validation ("Message not initialized") the moment the trigger runs
        // (bevy-traps.md #1 / #4). IDEMPOTENT with `wire_act_log`'s own registration on the
        // consumer side — the same both-sides shape SlabDestroyed / GroundAccrued already use.
        .add_message::<InterruptDeclared>()
        // GTW-526: the presenter-facing suppression signal `apply_suppression` emits once
        // per ganger freshly suppressed this tick (an idempotent refresh emits nothing).
        // Registering the buffer here makes `apply_suppression`'s
        // MessageWriter<SuppressionApplied> param valid and creates the
        // Messages<SuppressionApplied> buffer the presenter's suppression FCT reader will
        // drain (bevy-traps.md #4 / #5).
        .add_message::<SuppressionApplied>();
}
