//! The **Modify** injury effect (GTW-405; GTW-550 one-file-per-effect) — its
//! [`StatDelta`] payload, the isolated [`ApplyModify`] behaviour that SUMS the delta
//! into the ledger's per-stat store, and its exact inverse heal.

use bevy::prelude::Deref;
use serde::Deserialize;

use super::{ApplyInjuryEffect, HealError, LedgerAccumulators};
use crate::injuries::StatTarget;

/// The signed stat delta one [`Modify`](super::InjuryEffect::Modify) applies to a
/// [`StatTarget`] (`docs/combat/resolution.md` injury tables).
///
/// Normally a debuff (negative — an injury weakens the ganger), but a boost
/// (positive) is legal, so the inner is **signed** `i8` (`-128..=127`, ample for any
/// authored single-stat shift). A no-bare-types newtype (a stat shift is a domain
/// value, never a bare integer): private inner + derived [`Deref`];
/// `#[serde(transparent)]` lets an authored effect name it as a bare RON number
/// (`amount: -2`). The summed-delta store widens to `i16` so many stacked deltas
/// can never overflow — see [`StatDeltaSum`](crate::injuries::StatDeltaSum).
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(transparent)]
pub struct StatDelta(i8);

impl StatDelta {
    /// Build a stat delta from its signed magnitude (negative = debuff, positive =
    /// boost).
    #[must_use]
    pub const fn new(amount: i8) -> Self {
        Self(amount)
    }

    /// The signed magnitude as a bare `i8` — a `const` accessor the summed-delta
    /// fold needs (the derived [`Deref`] is not usable in a `const fn`).
    #[must_use]
    pub const fn raw(self) -> i8 {
        self.0
    }
}

/// **Modify** — folds a signed [`StatDelta`] into the ledger's per-stat summed-delta
/// store for one [`StatTarget`] (the modifier-layer fold the GTW-436 projector re-sums
/// every projection, so a `stat.tuning.ron` hot-reload re-applies it rather than wiping
/// it). Applies to BOTH the eight attributes and the eight derived stats; for a pool
/// target the projector docks the MAX.
pub struct ApplyModify {
    /// Which stat the delta shifts (attribute or derived).
    stat:   StatTarget,
    /// The signed shift (debuff negative, boost positive).
    amount: StatDelta,
}

impl ApplyModify {
    /// Build the modify effect from its target stat and signed delta.
    #[must_use]
    pub const fn new(stat: StatTarget, amount: StatDelta) -> Self {
        Self { stat, amount }
    }
}

impl ApplyInjuryEffect for ApplyModify {
    fn fold_on_gain(&self, accumulators: &mut LedgerAccumulators<'_>) {
        accumulators.deltas.add_delta(self.stat, self.amount);
    }

    /// The EXACT inverse of the gain fold: subtract the same delta from the same
    /// stat's running sum (saturating — exact unless the `i16` sum ever saturated,
    /// which takes 256+ worst-case same-sign stacked deltas).
    fn heal(&self, accumulators: &mut LedgerAccumulators<'_>) -> Result<(), HealError> {
        accumulators.deltas.remove_delta(self.stat, self.amount);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ApplyInjuryEffect, ApplyModify, StatDelta};
    use crate::{
        effects::injuries::LedgerAccumulators,
        injuries::{BleedAfflicted, MovementCostFactor, StatDeltaLedger, StatTarget},
    };

    /// `ApplyModify` SUMS its delta into the named stat's running sum — and ONLY that
    /// stat's (a leak into a sibling stat would break the assert on `Speed`).
    #[test]
    fn modify_folds_into_the_named_stat_sum() {
        let mut deltas = StatDeltaLedger::default();
        let mut bleed = BleedAfflicted::default();
        let mut movement = MovementCostFactor::IDENTITY;
        let mut acc = LedgerAccumulators {
            deltas:   &mut deltas,
            bleed:    &mut bleed,
            movement: &mut movement,
        };
        ApplyModify::new(StatTarget::Aim, StatDelta::new(-3)).fold_on_gain(&mut acc);
        assert_eq!(
            *deltas.delta_for(StatTarget::Aim),
            -3,
            "the delta sums into the named stat"
        );
        assert_eq!(
            *deltas.delta_for(StatTarget::Speed),
            0,
            "no other stat is touched"
        );
    }

    /// `heal` is the EXACT inverse of the gain fold: gain then heal restores the
    /// stat's running sum to its pre-gain value.
    #[test]
    fn heal_restores_the_pre_gain_sum() {
        let mut deltas = StatDeltaLedger::default();
        let mut bleed = BleedAfflicted::default();
        let mut movement = MovementCostFactor::IDENTITY;
        let mut acc = LedgerAccumulators {
            deltas:   &mut deltas,
            bleed:    &mut bleed,
            movement: &mut movement,
        };
        let effect = ApplyModify::new(StatTarget::Toughness, StatDelta::new(-4));
        effect.fold_on_gain(&mut acc);
        let healed = effect.heal(&mut acc);
        assert_eq!(healed, Ok(()), "Modify heals by exact inverse fold");
        assert_eq!(
            *deltas.delta_for(StatTarget::Toughness),
            0,
            "gain then heal restores the running sum"
        );
    }
}
