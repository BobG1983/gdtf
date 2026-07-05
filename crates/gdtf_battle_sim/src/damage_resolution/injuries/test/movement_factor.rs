//! GTW-444 — the `MovementCostMul` multiplicative fold (C2 / C4 / C5 / C6).

use super::{
    super::{
        BleedAmount, InflictedInjuries, InjuryDef, InjuryEffect, MovementCostFactor, StatDelta,
        StatTarget,
    },
    support::*,
};
use crate::armor::{BodyPart, InjuryCategory};

/// `f32` equality on a movement factor's inner — exact (the values under test are exactly
/// representable: 1.0, 1.5, 2.0, 3.0), so `==` is sound here.
fn factor_eq(a: MovementCostFactor, b: MovementCostFactor, msg: &str) {
    assert!(
        (a.raw() - b.raw()).abs() < f32::EPSILON,
        "{msg}: {a:?} != {b:?}"
    );
}

#[test]
fn movement_cost_factor_none_is_identity() {
    // C5 (none -> 1.0) + C6 identity: an empty ledger reports the IDENTITY factor (1.0),
    // so an uninjured mover is never accidentally slowed.
    let ledger = InflictedInjuries::default();
    factor_eq(
        ledger.movement_cost_factor(),
        MovementCostFactor::IDENTITY,
        "no injury -> identity (1.0)",
    );
}

#[test]
fn movement_cost_factor_one_mul_is_that_factor() {
    // C5 (one 1.5 -> 1.5): a single MovementCostMul(1.5) folds to exactly 1.5.
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(
        BodyPart::LeftLeg,
        vec![InjuryEffect::MovementCostMul(MovementCostFactor::new(1.5))],
    ));
    factor_eq(
        ledger.movement_cost_factor(),
        MovementCostFactor::new(1.5),
        "one MovementCostMul(1.5) -> 1.5",
    );
}

#[test]
fn movement_cost_factor_two_muls_multiply_not_add() {
    // C4 / C5 — THE SHARPEST PIN: two MovementCostMul (1.5, 2.0) MULTIPLY to 3.0, NOT add
    // to 3.5 and NOT sum to 3.5. An additive fold would FAIL this (it would give 3.5).
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(
        BodyPart::LeftLeg,
        vec![InjuryEffect::MovementCostMul(MovementCostFactor::new(1.5))],
    ));
    ledger.gain(gained_on(
        BodyPart::RightLeg,
        vec![InjuryEffect::MovementCostMul(MovementCostFactor::new(2.0))],
    ));
    factor_eq(
        ledger.movement_cost_factor(),
        MovementCostFactor::new(3.0),
        "two MovementCostMul (1.5, 2.0) MULTIPLY to 3.0 (NOT add to 3.5)",
    );
}

#[test]
fn movement_cost_factor_stacking_is_order_independent() {
    // C4: multiplication is commutative — gaining (2.0, 1.5) gives the SAME 3.0 as
    // (1.5, 2.0). Order-independent, deterministic.
    let mut a = InflictedInjuries::default();
    a.gain(gained_on(
        BodyPart::LeftLeg,
        vec![InjuryEffect::MovementCostMul(MovementCostFactor::new(2.0))],
    ));
    a.gain(gained_on(
        BodyPart::RightLeg,
        vec![InjuryEffect::MovementCostMul(MovementCostFactor::new(1.5))],
    ));
    factor_eq(
        a.movement_cost_factor(),
        MovementCostFactor::new(3.0),
        "stacking order does not matter (2.0 then 1.5 == 3.0)",
    );
}

#[test]
fn movement_cost_factor_other_effects_are_inert() {
    // C5 (a non-MovementCostMul effect -> 1.0): a Bleeding and a DisableHand (and a Modify)
    // contribute NOTHING to the movement factor — it stays IDENTITY. Pin-discriminating: a
    // fold that touched the movement accumulator for the wrong variant would FAIL.
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(
        BodyPart::LeftArm,
        vec![InjuryEffect::DisableHand],
    ));
    ledger.gain(gained_with(vec![InjuryEffect::Bleeding {
        amount: BleedAmount::new(2),
    }]));
    ledger.gain(gained_with(vec![InjuryEffect::Modify {
        stat:   StatTarget::Aim,
        amount: StatDelta::new(-2),
    }]));
    factor_eq(
        ledger.movement_cost_factor(),
        MovementCostFactor::IDENTITY,
        "Bleeding / DisableHand / Modify must leave the movement factor at identity",
    );
}

#[test]
fn movement_cost_mul_deserializes_from_ron() {
    // C1: the MovementCostMul(<f32>) variant parses from an effects Vec — the .injury.ron
    // shape the Hampered leg injuries use (the authored factor is a bare RON number).
    let ron = r#"(
        name:         "Shattered Knee",
        category:     Leg,
        severity:     Major,
        popup_text:   "KNEE SHATTERED",
        log_text:     "shatters a knee",
        inspect_text: "Shattered Knee -- Hampered movement",
        effects: [
            MovementCostMul(1.5),
            Modify(stat: Speed, amount: -1),
        ],
    )"#;
    let parsed = ron::from_str::<InjuryDef>(ron);
    assert!(
        parsed.is_ok(),
        "the MovementCostMul effect must deserialize: {parsed:?}"
    );
    let Ok(def) = parsed else {
        return;
    };
    assert_eq!(def.category, InjuryCategory::Leg);
    assert!(matches!(def.effects[0], InjuryEffect::MovementCostMul(_)));
}
