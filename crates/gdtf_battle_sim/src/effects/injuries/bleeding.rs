//! Bleeding injury effect.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::{ApplyInjuryEffect, HealError, LedgerAccumulators};

/// Per-turn bleed amount from an injury.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct BleedAmount(u8);

impl BleedAmount {
    /// Wrap an amount.
    #[must_use]
    pub const fn new(amount: u8) -> Self {
        Self(amount)
    }

    /// Inner value.
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
}

/// Accrues and relieves bleed on the ledger.
pub struct ApplyBleeding {
    amount: BleedAmount,
}

impl ApplyBleeding {
    /// Build the applicator.
    #[must_use]
    pub const fn new(amount: BleedAmount) -> Self {
        Self { amount }
    }
}

impl ApplyInjuryEffect for ApplyBleeding {
    fn fold_on_gain(&self, accumulators: &mut LedgerAccumulators<'_>) {
        *accumulators.bleed = accumulators.bleed.accumulate(self.amount);
    }

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
