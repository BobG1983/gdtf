//! Closed set of injury effects from injury defs.

use serde::{Deserialize, Serialize};

use super::{
    ApplyBleeding, ApplyDisableHand, ApplyInjuryEffect, ApplyModify, ApplyMovementCostMul,
    BleedAmount, HandDisabling, HealError, LedgerAccumulators, MovementCostFactor, StatDelta,
};
use crate::injuries::StatTarget;

/// One effect an injury can apply.
#[derive(Clone, Copy, PartialEq, Debug, Deserialize, Serialize)]
pub enum InjuryEffect {
    /// Change a named stat.
    Modify {
        /// Which stat.
        stat: StatTarget,
        /// Delta amount.
        amount: StatDelta,
    },
    /// Accrue bleed damage per turn.
    Bleeding {
        /// Bleed amount.
        amount: BleedAmount,
    },
    /// Disable a hand.
    DisableHand,
    /// Multiply movement cost.
    MovementCostMul(MovementCostFactor),
}

impl InjuryEffect {
    fn with_behaviour<R>(self, visit: impl FnOnce(&dyn ApplyInjuryEffect) -> R) -> R {
        match self {
            Self::Modify { stat, amount } => visit(&ApplyModify::new(stat, amount)),
            Self::Bleeding { amount } => visit(&ApplyBleeding::new(amount)),
            Self::DisableHand => visit(&ApplyDisableHand),
            Self::MovementCostMul(factor) => visit(&ApplyMovementCostMul::new(factor)),
        }
    }
}

impl ApplyInjuryEffect for InjuryEffect {
    fn fold_on_gain(&self, accumulators: &mut LedgerAccumulators<'_>) {
        self.with_behaviour(|behaviour| behaviour.fold_on_gain(accumulators));
    }

    fn disables_hand(&self) -> HandDisabling {
        self.with_behaviour(|behaviour| behaviour.disables_hand())
    }

    fn heal(&self, accumulators: &mut LedgerAccumulators<'_>) -> Result<(), HealError> {
        self.with_behaviour(|behaviour| behaviour.heal(accumulators))
    }
}
