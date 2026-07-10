//! Palette-level behaviour suite for the GTW-550 injury-effect palette: the closed
//! [`InjuryEffect`](super::InjuryEffect) vocabulary parses from RON by variant name
//! (the serde bridge), and the enum's THIN delegation `impl ApplyInjuryEffect` routes
//! each variant to its isolated behaviour (folded directly through the trait against a
//! [`LedgerAccumulators`](super::LedgerAccumulators) view, the seam the ledger's `gain`
//! wraps).
//!
//! Per-effect fold/heal semantics are asserted in each effect file's own
//! `#[cfg(test)]`; this suite proves the enum bridge (parse + delegation), not the
//! individual accumulator maths. Per the brittle-test rule, assertions check the
//! MAPPING + DIRECTION, never a shipped magnitude.

use super::{ApplyInjuryEffect, InjuryEffect, LedgerAccumulators, MovementCostFactor, StatDelta};
use crate::injuries::{BleedAfflicted, StatDeltaLedger, StatTarget};

/// The closed [`InjuryEffect`] vocabulary deserializes each variant from RON by name,
/// with payloads authored as bare scalars (the `#[serde(transparent)]` newtype bridge)
/// — the exact `.injury.ron` authoring forms. Pin-discriminating: a mis-named variant
/// or a wrong payload shape fails to parse. Asserts the MAPPING, not a magnitude.
#[test]
fn each_effect_variant_parses_from_ron() {
    // Modify authors as a struct variant with a named stat + bare delta.
    let Ok(modify) = ron::de::from_str::<InjuryEffect>("Modify(stat: Aim, amount: -2)") else {
        unreachable!("Modify(stat:, amount:) must parse");
    };
    assert!(
        matches!(modify, InjuryEffect::Modify { .. }),
        "Modify maps to the Modify variant"
    );

    // Bleeding authors as a struct variant with a bare amount.
    let Ok(bleeding) = ron::de::from_str::<InjuryEffect>("Bleeding(amount: 1)") else {
        unreachable!("Bleeding(amount:) must parse");
    };
    assert!(
        matches!(bleeding, InjuryEffect::Bleeding { .. }),
        "Bleeding maps to the Bleeding variant"
    );

    // DisableHand authors as a bare unit variant.
    assert!(
        matches!(
            ron::de::from_str::<InjuryEffect>("DisableHand"),
            Ok(InjuryEffect::DisableHand)
        ),
        "the no-payload DisableHand must parse as a unit variant"
    );

    // MovementCostMul authors as a bare f32 factor.
    let Ok(mul) = ron::de::from_str::<InjuryEffect>("MovementCostMul(1.5)") else {
        unreachable!("MovementCostMul(<f32>) must parse");
    };
    assert!(
        matches!(mul, InjuryEffect::MovementCostMul(_)),
        "MovementCostMul maps to the MovementCostMul variant"
    );
}

/// The [`InjuryEffect`] enum's THIN delegation `impl ApplyInjuryEffect` routes a
/// variant to its isolated behaviour — folding `Modify` through the enum moves the
/// same stat sum the isolated `ApplyModify` does, and the projection verb answers
/// `true` for exactly the hand-disabling variant. Proves the bridge forwards, not
/// that the enum carries logic.
#[test]
fn enum_delegates_to_the_isolated_behaviour() {
    let mut deltas = StatDeltaLedger::default();
    let mut bleed = BleedAfflicted::default();
    let mut movement = MovementCostFactor::IDENTITY;
    let mut acc = LedgerAccumulators {
        deltas:   &mut deltas,
        bleed:    &mut bleed,
        movement: &mut movement,
    };
    InjuryEffect::Modify {
        stat:   StatTarget::Aim,
        amount: StatDelta::new(-2),
    }
    .fold_on_gain(&mut acc);
    assert_eq!(
        *deltas.delta_for(StatTarget::Aim),
        -2,
        "the enum delegates Modify to ApplyModify (the named stat sum moved)"
    );

    // The projection verb delegates too: true for DisableHand, false elsewhere.
    assert!(
        *InjuryEffect::DisableHand.disables_hand(),
        "DisableHand projects the disabled hand through the trait"
    );
    assert!(
        !*InjuryEffect::MovementCostMul(MovementCostFactor::new(1.5)).disables_hand(),
        "a non-hand effect keeps the defaulted false projection"
    );
}
