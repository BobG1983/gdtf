//! The per-pass **pending-spend ledger** (GTW-646) — the trigger's working view of each
//! reactor's TU / facing / magazine, advanced as interrupts are EMITTED.
//!
//! The interrupt spend is settled LATER in the same tick:
//! [`reaction_trigger`](super::trigger::reaction_trigger) runs
//! `.before(dispatch_fire)`, so the TU / facing / magazine mutations of an emitted
//! interrupt shot land only when [`dispatch_fire`](crate::acts::dispatch_fire) →
//! `fire()` consumes the [`FireRequested`](crate::acts::FireRequested). With a
//! reaction cap above one and SEVERAL actors acting in one tick, a reactor can be
//! evaluated more than once per pass — and every evaluation after a successful
//! interrupt would otherwise gate on the SETTLED snapshot, offering (and counting,
//! via [`ReactionsUsed`](crate::tuning::ReactionsUsed)) an interrupt the dispatcher
//! then rejects as unaffordable: a reaction spent on a non-shot (the GTW-646 filed
//! defect).
//!
//! This ledger closes that gap WITHOUT moving the spend's owner: `try_reaction`
//! still increments the cap counter at emission, but its gates read the reactor's
//! state through the ledger — the settled snapshot OVERLAID with every spend already
//! committed this pass, computed with the SAME shared verdict sources the dispatcher
//! runs ([`decide_fire_arc`](crate::acts::decide_fire_arc) for turn cost + facing,
//! [`mode_tu_cost`](crate::magazine::mode_tu_cost) for the shot charge, and the
//! [`Magazine`] arithmetic `fire()` applies). An offer the dispatcher could not
//! accept is therefore skipped in the ELIGIBILITY gates, BEFORE the opposed-check
//! roll — consuming zero [`ReactionRng`](crate::rng::ReactionRng) draws, the same
//! stream-preserving skip shape as the GTW-526 suppression gate.

use bevy::{platform::collections::HashMap, prelude::Entity};

use crate::{
    ganger::{Facing, Tu},
    magazine::Magazine,
    tu::spend_tu,
    weapon::ModeShots,
};

/// The state ONE successful interrupt commits against its reactor — the post-dispatch
/// TU / facing / magazine `try_reaction` predicts from the dispatcher's own shared
/// verdict functions, handed back to the trigger to fold into the
/// [`PendingSpendLedger`].
pub(super) struct InterruptCommit {
    /// The reactor that fired the interrupt — the ledger overlay's key.
    reactor:        Entity,
    /// The reactor's wielded ranged-weapon entity — the magazine overlay's key.
    weapon:         Entity,
    /// The reactor's TU pool AFTER the committed turn + fire charges.
    tu_after:       Tu,
    /// The reactor's facing AFTER the committed turn-into-arc (unchanged in-arc).
    facing_after:   Facing,
    /// The weapon's magazine AFTER the committed rounds are spent.
    magazine_after: Magazine,
}

impl InterruptCommit {
    /// Predict the post-dispatch reactor state of one emitted interrupt from the
    /// spends the dispatcher will apply: charge `spend` (the arc verdict's turn cost
    /// plus the shared mode charge) against the WORKING pool `tu_now` (the same
    /// saturating [`spend_tu`] the TU economy runs), spend `rounds` (the `fire()`
    /// burst clamp's count) from the WORKING `magazine`, and adopt the post-turn
    /// `facing_after`.
    #[must_use]
    pub(super) fn predict(
        reactor: Entity,
        weapon: Entity,
        tu_now: Tu,
        spend: Tu,
        facing_after: Facing,
        rounds: ModeShots,
        magazine: Magazine,
    ) -> Self {
        let mut tu_after = tu_now;
        spend_tu(&mut tu_after, spend);
        let mut magazine_after = magazine;
        for _ in 0..*rounds {
            magazine_after.spend_round();
        }
        Self {
            reactor,
            weapon,
            tu_after,
            facing_after,
            magazine_after,
        }
    }
}

/// A reactor's committed TU + facing overlay — the working values every evaluation
/// AFTER its successful interrupt this pass gates on (in place of the settled
/// snapshot row).
struct ReactorOverlay {
    /// The reactor's TU pool net of every spend committed this pass.
    tu:     Tu,
    /// The reactor's facing after every turn-into-arc committed this pass.
    facing: Facing,
}

/// The per-pass pending-spend ledger — settled state overlaid with the spends of
/// every interrupt already emitted this trigger pass (see the module docs for why).
///
/// One instance lives per [`reaction_trigger`](super::trigger::reaction_trigger)
/// run (a plain local, never a `Resource` — the pending spends are meaningless once
/// `dispatch_fire` settles them later the same tick).
#[derive(Default)]
pub(super) struct PendingSpendLedger {
    /// Per-reactor TU/facing overlays, keyed by the reactor entity.
    overlays:  HashMap<Entity, ReactorOverlay>,
    /// Per-weapon working magazines (rounds net of committed shots), keyed by the
    /// weapon entity.
    magazines: HashMap<Entity, Magazine>,
}

impl PendingSpendLedger {
    /// The reactor's WORKING TU — its committed overlay, or `settled` (the snapshot
    /// row's value) when nothing was committed against it this pass.
    #[must_use]
    pub(super) fn tu_of(&self, reactor: Entity, settled: Tu) -> Tu {
        self.overlays
            .get(&reactor)
            .map_or(settled, |overlay| overlay.tu)
    }

    /// The reactor's WORKING facing — its committed overlay, or `settled` (the
    /// snapshot row's value) when nothing was committed against it this pass.
    #[must_use]
    pub(super) fn facing_of(&self, reactor: Entity, settled: Facing) -> Facing {
        self.overlays
            .get(&reactor)
            .map_or(settled, |overlay| overlay.facing)
    }

    /// The weapon's WORKING magazine — its committed overlay, or `live` (a copy of
    /// the weapon entity's current component) when no shot was committed from it
    /// this pass.
    #[must_use]
    pub(super) fn magazine_of(&self, weapon: Entity, live: Magazine) -> Magazine {
        self.magazines
            .get(&weapon)
            .map_or(live, |magazine| *magazine)
    }

    /// Fold one emitted interrupt's predicted post-dispatch state into the ledger,
    /// so every later evaluation this pass gates on it.
    pub(super) fn commit(&mut self, commit: InterruptCommit) {
        self.overlays.insert(
            commit.reactor,
            ReactorOverlay {
                tu:     commit.tu_after,
                facing: commit.facing_after,
            },
        );
        self.magazines.insert(commit.weapon, commit.magazine_after);
    }
}
