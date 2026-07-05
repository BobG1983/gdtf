//! Reaction-fire + suppression wiring — the GTW-468 reaction trigger + cap reset
//! and the GTW-526 suppression producer / auto-stance / clear cadence.

use bevy::prelude::{App, IntoScheduleConfigs, Update};

use crate::{
    acts::fire::dispatch_fire,
    move_acts::advance_walk,
    occupancy_sync::SimSystems,
    reaction::{reaction_trigger, reset_reactions_used},
    suppression::{apply_suppression, reset_suppression, suppression_auto_stance},
    turn::dispatch_end_turn,
};

/// Wire the reaction-fire trigger, its turn-boundary cap reset, and the suppression
/// producer / auto-stance / clear systems into the [`SimSystems::Simulate`] band — the
/// reaction/suppression half of [`SimActsPlugin::build`](super::SimActsPlugin)'s wiring
/// (each `add_systems` call is independent; ordering is carried by
/// `.before`/`.after`/`.run_if`, not call order).
pub(super) fn wire_reaction_suppression(app: &mut App) {
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
    app.add_systems(
        Update,
        reaction_trigger
            .before(dispatch_fire)
            .before(advance_walk)
            .in_set(SimSystems::Simulate),
    );
    // GTW-468 (C6): the turn-boundary cap reset. `dispatch_end_turn` emits a
    // TurnStarted at each advance; this zeroes every watcher's ReactionsUsed so the §8
    // per-turn cap applies AFRESH each turn (generalizing "this enemy turn" to "this
    // turn relative to the watcher" — reset every boundary is the defensible default,
    // documented on the system). Ordered `.after(dispatch_end_turn)` so the boundary's
    // TurnStarted is buffered (its own independent reader, so it never steals the
    // boundary from the combat-log / bleed readers — the `tick_bleed` precedent). It
    // joins the gated Simulate band; its Query + MessageReader are always valid, so it
    // needs no extra run_if. Param-only, no &mut World (bevy-traps.md #7).
    app.add_systems(
        Update,
        reset_reactions_used
            .after(dispatch_end_turn)
            .in_set(SimSystems::Simulate),
    );
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
    app.add_systems(
        Update,
        apply_suppression
            .after(dispatch_fire)
            .in_set(SimSystems::Simulate),
    );
    // GTW-526 (C5): the AUTO-STANCE drop. On a fresh Added<Suppressed> it ducks the ganger
    // behind the cover one step toward the suppressor, writing Stance DIRECTLY (no TU).
    // Ordered `.after(apply_suppression)` (bevy-traps.md #3) so the producer's deferred
    // Commands insert of Suppressed is FLUSHED (the explicit ordering forces a sync point)
    // and this frame's Added<Suppressed> is detected the SAME tick the unit is suppressed.
    // Param-only (Query + Res<CoverLedger>) — the ledger is battle-lifetime, so it joins
    // the gated Simulate band (the run_if skips it outside a live battle, bevy-traps.md #1).
    app.add_systems(
        Update,
        suppression_auto_stance
            .after(apply_suppression)
            .in_set(SimSystems::Simulate),
    );
    // GTW-526 (C6): the CLEAR cadence. On a TurnStarted it removes Suppressed from every
    // ganger of the now-active faction (faction-scoped, so a unit stays pinned through the
    // opponent's turn and clears at its OWN turn-start — NOT the faction-agnostic
    // reset_reactions_used cadence). Ordered `.after(dispatch_end_turn)` so the boundary's
    // TurnStarted is buffered (its own independent reader, so it never steals the boundary
    // from the combat-log / bleed / cap-reset readers — the reset_reactions_used
    // precedent). It joins the gated Simulate band; its MessageReader + Query + Commands
    // are always valid, so it needs no extra run_if. Param-only, no &mut World
    // (bevy-traps.md #7).
    app.add_systems(
        Update,
        reset_suppression
            .after(dispatch_end_turn)
            .in_set(SimSystems::Simulate),
    );
}
