//! Movement-cost multiplier injury effect.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::{ApplyInjuryEffect, HealError, LedgerAccumulators};

/// Multiplier applied to movement cost (1.0 = no change).
#[derive(Deref, Clone, Copy, PartialEq, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct MovementCostFactor(f32);

impl MovementCostFactor {
    /// No change.
    pub const IDENTITY: Self = Self(1.0);

    /// Wrap a factor.
    #[must_use]
    pub const fn new(factor: f32) -> Self {
        Self(factor)
    }

    /// Multiply two factors.
    #[must_use]
    pub fn times(self, other: Self) -> Self {
        Self(self.0 * other.0)
    }

    /// Inner value.
    #[must_use]
    pub const fn raw(self) -> f32 {
        self.0
    }
}

impl Default for MovementCostFactor {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Multiplies the movement cost factor on gain; heal requires a full refold.
pub struct ApplyMovementCostMul {
    factor: MovementCostFactor,
}

impl ApplyMovementCostMul {
    /// Build the applicator.
    #[must_use]
    pub const fn new(factor: MovementCostFactor) -> Self {
        Self { factor }
    }
}

impl ApplyInjuryEffect for ApplyMovementCostMul {
    fn fold_on_gain(&self, accumulators: &mut LedgerAccumulators<'_>) {
        *accumulators.movement = accumulators.movement.times(self.factor);
    }

    fn heal(&self, _accumulators: &mut LedgerAccumulators<'_>) -> Result<(), HealError> {
        Err(HealError::NeedsRefold)
    }
}

#[cfg(test)]
mod tests {
    use super::{ApplyInjuryEffect, ApplyMovementCostMul, HealError, MovementCostFactor};
    use crate::{
        effects::injuries::LedgerAccumulators,
        injuries::{BleedAfflicted, StatDeltaLedger},
    };

    #[test]
    fn movement_cost_mul_stacks_multiplicatively() {
        let mut deltas = StatDeltaLedger::default();
        let mut bleed = BleedAfflicted::default();
        let mut movement = MovementCostFactor::IDENTITY;
        let mut acc = LedgerAccumulators {
            deltas: &mut deltas,
            bleed: &mut bleed,
            movement: &mut movement,
        };
        ApplyMovementCostMul::new(MovementCostFactor::new(1.5)).fold_on_gain(&mut acc);
        ApplyMovementCostMul::new(MovementCostFactor::new(2.0)).fold_on_gain(&mut acc);
        assert_eq!(
            movement.raw().to_bits(),
            3.0_f32.to_bits(),
            "1.5 × 2.0 folds to exactly 3.0 (multiplied, not added)"
        );
    }

    #[test]
    fn heal_signals_refold_and_leaves_the_product_untouched() {
        let mut deltas = StatDeltaLedger::default();
        let mut bleed = BleedAfflicted::default();
        let mut movement = MovementCostFactor::IDENTITY;
        let mut acc = LedgerAccumulators {
            deltas: &mut deltas,
            bleed: &mut bleed,
            movement: &mut movement,
        };
        let effect = ApplyMovementCostMul::new(MovementCostFactor::new(1.5));
        effect.fold_on_gain(&mut acc);
        let healed = effect.heal(&mut acc);
        assert_eq!(
            healed,
            Err(HealError::NeedsRefold),
            "an f32 product cannot be exactly inverse-folded"
        );
        assert_eq!(
            movement.raw().to_bits(),
            1.5_f32.to_bits(),
            "a refused heal leaves the accumulator untouched"
        );
    }
}
