//! Disable-hand injury effect.

use super::{ApplyInjuryEffect, HandDisabling, HealError, LedgerAccumulators};

/// Marks the injury as disabling a hand (no ledger mutation).
pub struct ApplyDisableHand;

impl ApplyInjuryEffect for ApplyDisableHand {
    fn fold_on_gain(&self, _accumulators: &mut LedgerAccumulators<'_>) {}

    fn disables_hand(&self) -> HandDisabling {
        HandDisabling::new(true)
    }

    fn heal(&self, _accumulators: &mut LedgerAccumulators<'_>) -> Result<(), HealError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ApplyDisableHand, ApplyInjuryEffect};
    use crate::{
        effects::injuries::LedgerAccumulators,
        injuries::{BleedAfflicted, MovementCostFactor, StatDeltaLedger, StatTarget},
    };

    #[test]
    fn disable_hand_is_inert_at_gain_and_projects_on_read() {
        let mut deltas = StatDeltaLedger::default();
        let mut bleed = BleedAfflicted::default();
        let mut movement = MovementCostFactor::IDENTITY;
        let mut acc = LedgerAccumulators {
            deltas:   &mut deltas,
            bleed:    &mut bleed,
            movement: &mut movement,
        };
        ApplyDisableHand.fold_on_gain(&mut acc);
        let healed = ApplyDisableHand.heal(&mut acc);
        assert_eq!(healed, Ok(()), "nothing to heal — a documented no-op Ok");
        assert!(
            *ApplyDisableHand.disables_hand(),
            "the projection is the behaviour"
        );
        assert_eq!(
            *deltas.delta_for(StatTarget::Strength),
            0,
            "no stat sum moves at gain"
        );
        assert_eq!(*bleed, 0, "no bleed accrues at gain");
        assert_eq!(
            movement.raw().to_bits(),
            MovementCostFactor::IDENTITY.raw().to_bits(),
            "the movement factor stays at identity"
        );
    }
}
