//! `#[cfg(test)]`; this suite proves the enum bridge (parse + delegation), not the
use super::{ApplyInjuryEffect, InjuryEffect, LedgerAccumulators, MovementCostFactor, StatDelta};
use crate::injuries::{BleedAfflicted, StatDeltaLedger, StatTarget};

/// with payloads authored as bare scalars (the `#[serde(transparent)]` newtype bridge)
#[test]
fn each_effect_variant_parses_from_ron() {
    let Ok(modify) = ron::de::from_str::<InjuryEffect>("Modify(stat: Aim, amount: -2)") else {
        unreachable!("Modify(stat:, amount:) must parse");
    };
    assert!(
        matches!(modify, InjuryEffect::Modify { .. }),
        "Modify maps to the Modify variant"
    );

    let Ok(bleeding) = ron::de::from_str::<InjuryEffect>("Bleeding(amount: 1)") else {
        unreachable!("Bleeding(amount:) must parse");
    };
    assert!(
        matches!(bleeding, InjuryEffect::Bleeding { .. }),
        "Bleeding maps to the Bleeding variant"
    );

    assert!(
        matches!(
            ron::de::from_str::<InjuryEffect>("DisableHand"),
            Ok(InjuryEffect::DisableHand)
        ),
        "the no-payload DisableHand must parse as a unit variant"
    );

    let Ok(mul) = ron::de::from_str::<InjuryEffect>("MovementCostMul(1.5)") else {
        unreachable!("MovementCostMul(<f32>) must parse");
    };
    assert!(
        matches!(mul, InjuryEffect::MovementCostMul(_)),
        "MovementCostMul maps to the MovementCostMul variant"
    );
}

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

    assert!(
        *InjuryEffect::DisableHand.disables_hand(),
        "DisableHand projects the disabled hand through the trait"
    );
    assert!(
        !*InjuryEffect::MovementCostMul(MovementCostFactor::new(1.5)).disables_hand(),
        "a non-hand effect keeps the defaulted false projection"
    );
}
