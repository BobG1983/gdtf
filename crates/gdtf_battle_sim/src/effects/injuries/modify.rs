//! Stat-modify injury effect.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::{ApplyInjuryEffect, HealError, LedgerAccumulators};
use crate::injuries::StatTarget;

/// Signed delta applied to a named stat.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct StatDelta(i8);

impl StatDelta {
    /// Wrap an amount.
    #[must_use]
    pub const fn new(amount: i8) -> Self {
        Self(amount)
    }

    /// Inner value.
    #[must_use]
    pub const fn raw(self) -> i8 {
        self.0
    }
}

/// Applies a stat delta on gain and reverses it on heal.
pub struct ApplyModify {
    stat:   StatTarget,
    amount: StatDelta,
}

impl ApplyModify {
    /// Build the applicator.
    #[must_use]
    pub const fn new(stat: StatTarget, amount: StatDelta) -> Self {
        Self { stat, amount }
    }
}

impl ApplyInjuryEffect for ApplyModify {
    fn fold_on_gain(&self, accumulators: &mut LedgerAccumulators<'_>) {
        accumulators.deltas.add_delta(self.stat, self.amount);
    }

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
