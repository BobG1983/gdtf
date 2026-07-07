//! The **`MovementCostMul`** injury effect (GTW-444 "Hampered"; GTW-550
//! one-file-per-effect) — its [`MovementCostFactor`] payload, the isolated
//! [`ApplyMovementCostMul`] behaviour that MULTIPLIES the ledger's movement
//! accumulator, and its refold-signalling heal.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::{ApplyInjuryEffect, HealError, LedgerAccumulators};

/// The **per-step movement TU-cost multiplier** one
/// [`MovementCostMul`](super::InjuryEffect::MovementCostMul) applies (GTW-444 — the
/// "Hampered" effect; `docs/combat/resolution.md` injury tables).
///
/// A factor `>= 1.0` means SLOWER (it MULTIPLIES the GTW-396 terrain per-step floor cost
/// up): `1.0` = no effect (the uninjured identity), `1.5` = each step costs 50 % more TU,
/// `2.0` = double. A no-bare-types newtype (a movement multiplier is a domain value,
/// distinct from any other `f32`): private inner + derived [`Deref`];
/// `#[serde(transparent)]` lets an authored effect name it as a bare RON number
/// (`MovementCostMul(1.5)`).
///
/// Inner `f32` (a continuous multiplier, not an integer count). It derives [`PartialEq`]
/// but NOT [`Eq`] / [`Hash`] (an `f32` has neither) — the same reason
/// [`InflictedInjuries`](crate::injuries::InflictedInjuries), which folds these into a
/// multiplicative accumulator field, drops its own `Eq` derive (the `f32` cannot be `Eq`;
/// a fixed-point rep would forfeit the exact, deterministic multiply this needs, so the
/// model keeps the `f32` and the `PartialEq`-only derive). The accumulated product folds
/// through [`InflictedInjuries::movement_cost_factor`](crate::injuries::InflictedInjuries::movement_cost_factor),
/// and the pathfinder + the committed walk both scale each per-step cost by it (the
/// preview==charge consistency, GTW-444 C3). `Serialize` is added (GTW-654) so the
/// content editor's INJURY authoring mode can write an edited effects list back to
/// disk (behavior-inert for the sim).
#[derive(Deref, Clone, Copy, PartialEq, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct MovementCostFactor(f32);

impl MovementCostFactor {
    /// The identity factor (`1.0`) — no slowdown. The default a ledger with no
    /// [`MovementCostMul`](super::InjuryEffect::MovementCostMul) reports, and the
    /// neutral element of the multiplicative fold.
    pub const IDENTITY: Self = Self(1.0);

    /// Build a movement-cost factor from its raw multiplier (`>= 1.0` = slower; `1.0` =
    /// no effect). A factor below `1.0` would speed the ganger up, which the "Hampered"
    /// semantics never author, but the type does not clamp — the authored data is
    /// expected to keep it `>= 1.0` (the `.injury.ron` convention).
    #[must_use]
    pub const fn new(factor: f32) -> Self {
        Self(factor)
    }

    /// The product of this factor and `other` — the multiplicative fold of two stacked
    /// [`MovementCostMul`](super::InjuryEffect::MovementCostMul) effects (GTW-444 C4).
    ///
    /// STACKING MULTIPLIES (the locked default — NOT additive, NOT capped): two leg
    /// injuries of `1.5` and `2.0` combine to `3.0`, NOT `3.5`. The running product is
    /// folded in ledger order, so it is deterministic for a fixed entry sequence (the
    /// GTW-441 authoring guide owes this rule to authors).
    #[must_use]
    pub fn times(self, other: Self) -> Self {
        Self(self.0 * other.0)
    }

    /// The raw multiplier as a bare `f32` — the accessor the per-step cost scaling reads
    /// (the derived [`Deref`] also yields it; this is the explicit form the cost-scale
    /// helper uses).
    #[must_use]
    pub const fn raw(self) -> f32 {
        self.0
    }
}

impl Default for MovementCostFactor {
    /// The identity factor (`1.0`) — no slowdown (an uninjured mover, or a ledger with
    /// no [`MovementCostMul`](super::InjuryEffect::MovementCostMul)).
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// **`MovementCostMul`** — MULTIPLIES its [`MovementCostFactor`] into the ledger's
/// dedicated movement accumulator (GTW-444 C2 — multiplicative, not additive; two
/// stacked factors of `1.5` and `2.0` fold to `3.0`). The pathfinder cost-function and
/// the committed walk's per-step TU charge both read the accumulated product, so a
/// Hampered unit's previewed path cost equals the TU actually charged (preview==charge,
/// GTW-444 C3). Part-agnostic: the fold does not key on the struck part.
pub struct ApplyMovementCostMul {
    /// The per-step multiplier this injury contributes to the running product.
    factor: MovementCostFactor,
}

impl ApplyMovementCostMul {
    /// Build the movement-cost effect from its per-step multiplier.
    #[must_use]
    pub const fn new(factor: MovementCostFactor) -> Self {
        Self { factor }
    }
}

impl ApplyInjuryEffect for ApplyMovementCostMul {
    fn fold_on_gain(&self, accumulators: &mut LedgerAccumulators<'_>) {
        *accumulators.movement = accumulators.movement.times(self.factor);
    }

    /// NON-INVERTIBLE: the accumulator is an `f32` running product, and IEEE-754
    /// division is not the exact algebraic inverse of the gain-time multiply — an
    /// incremental "divide it back out" could drift the product by an ulp and break
    /// the deterministic preview==charge contract. The healer must re-fold the
    /// remaining ledger entries instead (the documented [`HealError::NeedsRefold`]
    /// fallback).
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

    /// `ApplyMovementCostMul` MULTIPLIES into the movement accumulator — two stacked
    /// folds of 1.5 and 2.0 land on exactly 3.0 (the locked multiplicative stacking).
    #[test]
    fn movement_cost_mul_stacks_multiplicatively() {
        let mut deltas = StatDeltaLedger::default();
        let mut bleed = BleedAfflicted::default();
        let mut movement = MovementCostFactor::IDENTITY;
        let mut acc = LedgerAccumulators {
            deltas:   &mut deltas,
            bleed:    &mut bleed,
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

    /// `heal` reports the non-invertible fold: the healer must refold the remaining
    /// entries, and the accumulator is left untouched by the refused heal.
    #[test]
    fn heal_signals_refold_and_leaves_the_product_untouched() {
        let mut deltas = StatDeltaLedger::default();
        let mut bleed = BleedAfflicted::default();
        let mut movement = MovementCostFactor::IDENTITY;
        let mut acc = LedgerAccumulators {
            deltas:   &mut deltas,
            bleed:    &mut bleed,
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
