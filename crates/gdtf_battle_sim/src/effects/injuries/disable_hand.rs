//! The **`DisableHand`** injury effect (GTW-443; GTW-550 one-file-per-effect) — the
//! isolated, payload-free [`ApplyDisableHand`] behaviour: INERT at gain, surfaced
//! through the read-side hand projection instead of any accumulator.

use super::{ApplyInjuryEffect, HealError, LedgerAccumulators};

/// **`DisableHand`** — disables the hand on the injury's struck arm (GTW-443).
///
/// INERT AT GAIN: it folds NOTHING into the accumulators. The disabled hand is a
/// READ-SIDE projection —
/// [`InflictedInjuries::hands_available`](crate::injuries::InflictedInjuries::hands_available)
/// folds the ledger's entries into a SET of distinct disabled arm-sides, asking each
/// effect [`disables_hand`](ApplyInjuryEffect::disables_hand) and keying the side on
/// the entry's struck [`part`](crate::injuries::GainedInjury::part)
/// ([`LeftArm`](crate::armor::BodyPart::LeftArm) → the left hand,
/// [`RightArm`](crate::armor::BodyPart::RightArm) → the right; a `DisableHand` carried
/// by a Head / Torso / Leg injury is INERT — no hand to disable). Read-fold, not
/// stored, so two same-side `DisableHand` injuries still disable exactly ONE hand (set
/// membership over sides, the single-source-of-truth a stored counter could not give
/// without de-duping).
///
/// The 1H aim PENALTY a hand-disabling injury also carries rides as a SEPARATE
/// [`Modify`](super::InjuryEffect::Modify)`(Shooting, -N)` effect in the same injury's
/// effects `Vec` — the penalty flows through the normal modifier layer while this
/// effect only gates two-handed fire (the `can_fire` hand-count clause).
pub struct ApplyDisableHand;

impl ApplyInjuryEffect for ApplyDisableHand {
    /// A documented NO-OP: nothing is accumulated at gain — the disabled hand is
    /// derived on read by the hand projection (see the type-level docs).
    fn fold_on_gain(&self, _accumulators: &mut LedgerAccumulators<'_>) {}

    /// THE hand-disabling effect — the one override of the defaulted projection.
    fn disables_hand(&self) -> bool {
        true
    }

    /// A documented NO-OP `Ok`: nothing was accumulated at gain, so there is nothing
    /// to inverse-fold — removing the healed
    /// [`GainedInjury`](crate::injuries::GainedInjury) entry from the ledger is itself
    /// the heal (the read-side projection stops seeing it).
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

    /// `ApplyDisableHand` is INERT at gain (no accumulator moves) and heals as a
    /// documented no-op `Ok` — its whole behaviour is the read-side projection.
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
            ApplyDisableHand.disables_hand(),
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
