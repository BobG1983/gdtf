//! The **Bleeding** injury effect (GTW-405/GTW-438; GTW-550 one-file-per-effect) — its
//! [`BleedAmount`] payload, the isolated [`ApplyBleeding`] behaviour that ACCRUES the
//! per-turn drain onto the ledger's bleed total, and its exact inverse heal.

use bevy::prelude::Deref;
use serde::Deserialize;

use super::{ApplyInjuryEffect, HealError, LedgerAccumulators};

/// The per-turn HP a [`Bleeding`](super::InjuryEffect::Bleeding) injury drains, through
/// the existing bleed runtime (`docs/combat/resolution.md` §9).
///
/// An unsigned `u8` (a bleed only ever drains; the per-turn amount is small). A
/// no-bare-types newtype (a bleed rate is a domain value): private inner + derived
/// [`Deref`]; `#[serde(transparent)]` parses a bare RON number (`amount: 1`). The
/// accrued total widens to `u16` so stacked bleeds can never overflow — see
/// [`BleedAfflicted`](crate::injuries::BleedAfflicted). Distinct from the Downed Wounds
/// bleed-out: this drains HP and can down but never kill.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(transparent)]
pub struct BleedAmount(u8);

impl BleedAmount {
    /// Build a per-turn bleed amount from its magnitude (HP drained each turn).
    #[must_use]
    pub const fn new(amount: u8) -> Self {
        Self(amount)
    }

    /// The magnitude as a bare `u8` — a `const` accessor the bleed accrual needs
    /// (the derived [`Deref`] is not usable in a `const fn`).
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
}

/// **Bleeding** — accrues a per-turn [`BleedAmount`] onto the ledger's summed bleed
/// total, which the [`apply_injury`](crate::acts::apply_injury) boundary mirrors onto
/// the standalone [`BleedAfflicted`](crate::injuries::BleedAfflicted) component the
/// bleed runtime ([`tick_bleed`](crate::bleed::tick_bleed)) drains each round. NOT a
/// [`Modify`](super::InjuryEffect::Modify): it drains the current HP pool, distinct
/// from any stat ceiling.
pub struct ApplyBleeding {
    /// The per-turn HP drained while the injury persists.
    amount: BleedAmount,
}

impl ApplyBleeding {
    /// Build the bleeding effect from its per-turn drain amount.
    #[must_use]
    pub const fn new(amount: BleedAmount) -> Self {
        Self { amount }
    }
}

impl ApplyInjuryEffect for ApplyBleeding {
    fn fold_on_gain(&self, accumulators: &mut LedgerAccumulators<'_>) {
        *accumulators.bleed = accumulators.bleed.accumulate(self.amount);
    }

    /// The EXACT inverse of the gain fold: relieve the same amount from the accrued
    /// bleed total (saturating — exact unless the `u16` accrual ever saturated).
    fn heal(&self, accumulators: &mut LedgerAccumulators<'_>) -> Result<(), HealError> {
        *accumulators.bleed = accumulators.bleed.relieve(self.amount);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ApplyBleeding, ApplyInjuryEffect, BleedAmount};
    use crate::{
        effects::injuries::LedgerAccumulators,
        injuries::{BleedAfflicted, MovementCostFactor, StatDeltaLedger},
    };

    /// `ApplyBleeding` ACCRUES onto the summed bleed total — two folds stack by
    /// summing (2 + 3 = 5), and the stat store is untouched.
    #[test]
    fn bleeding_accrues_by_summing() {
        let mut deltas = StatDeltaLedger::default();
        let mut bleed = BleedAfflicted::default();
        let mut movement = MovementCostFactor::IDENTITY;
        let mut acc = LedgerAccumulators {
            deltas:   &mut deltas,
            bleed:    &mut bleed,
            movement: &mut movement,
        };
        ApplyBleeding::new(BleedAmount::new(2)).fold_on_gain(&mut acc);
        ApplyBleeding::new(BleedAmount::new(3)).fold_on_gain(&mut acc);
        assert_eq!(*bleed, 5, "stacked bleeds sum (2 + 3 = 5)");
    }

    /// `heal` is the EXACT inverse of the gain fold: gain then heal restores the
    /// accrued bleed to its pre-gain value.
    #[test]
    fn heal_relieves_the_accrued_bleed() {
        let mut deltas = StatDeltaLedger::default();
        let mut bleed = BleedAfflicted::default();
        let mut movement = MovementCostFactor::IDENTITY;
        let mut acc = LedgerAccumulators {
            deltas:   &mut deltas,
            bleed:    &mut bleed,
            movement: &mut movement,
        };
        let effect = ApplyBleeding::new(BleedAmount::new(4));
        effect.fold_on_gain(&mut acc);
        let healed = effect.heal(&mut acc);
        assert_eq!(healed, Ok(()), "Bleeding heals by exact inverse fold");
        assert_eq!(*bleed, 0, "gain then heal restores the accrual");
    }
}
