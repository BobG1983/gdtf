//! GTW-443 — `DisableHand` inert gain (C2) + the per-side `hands_available` fold (C3).

use super::{
    super::{
        BleedAfflicted, HandsAvailable, InflictedInjuries, InjuryDef, InjuryEffect, StatDeltaSum,
        StatTarget,
    },
    support::*,
};
use crate::armor::{BodyPart, InjuryCategory};

#[test]
fn gain_of_disable_hand_is_inert_no_delta_no_bleed() {
    // C2: a DisableHand-bearing injury folds INERTLY — it accumulates NO stat delta and
    // NO bleed by that effect alone. (The hand count is surfaced by the read-fold, not a
    // stored accumulator.) Pin-discriminating: a stored-counter implementation would
    // dirty some StatDeltaSum or the bleed here.
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(
        BodyPart::LeftArm,
        vec![InjuryEffect::DisableHand],
    ));

    // Every stat delta stays at its zero default, and the bleed accrual is untouched.
    for stat in StatTarget::ALL {
        assert_eq!(
            ledger.delta_for(stat),
            StatDeltaSum::default(),
            "DisableHand must not dock {stat:?}"
        );
    }
    assert_eq!(
        ledger.bleed(),
        BleedAfflicted::default(),
        "DisableHand must accrue no bleed"
    );
    // The ledger still RECORDS the injury (the fold reads it).
    assert_eq!(ledger.gained().len(), 1);
}

#[test]
fn hands_available_none_is_two() {
    // C3(a): no arm injury → both hands available.
    let ledger = InflictedInjuries::default();
    assert_eq!(ledger.hands_available(), HandsAvailable::new(2));
}

#[test]
fn hands_available_one_left_arm_disable_is_one() {
    // C3(b): one LeftArm DisableHand → exactly one hand left.
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(
        BodyPart::LeftArm,
        vec![InjuryEffect::DisableHand],
    ));
    assert_eq!(ledger.hands_available(), HandsAvailable::new(1));
}

#[test]
fn two_same_side_disable_hands_still_leave_one_hand() {
    // C3(c) — THE SHARPEST PIN: two LeftArm DisableHand injuries disable exactly ONE
    // hand, leaving 1 (NOT 0). A naive injury-count subtraction (2 - 2 = 0) would FAIL
    // this; the set-over-distinct-sides fold gives 1. Order-independent.
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(
        BodyPart::LeftArm,
        vec![InjuryEffect::DisableHand],
    ));
    ledger.gain(gained_on(
        BodyPart::LeftArm,
        vec![InjuryEffect::DisableHand],
    ));
    assert_eq!(
        ledger.hands_available(),
        HandsAvailable::new(1),
        "two SAME-side DisableHand injuries disable exactly one hand (set over sides, \
         not a count over entries)"
    );
}

#[test]
fn both_arms_disabled_is_zero_hands() {
    // C3(d): LeftArm + RightArm DisableHand → zero hands.
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(
        BodyPart::LeftArm,
        vec![InjuryEffect::DisableHand],
    ));
    ledger.gain(gained_on(
        BodyPart::RightArm,
        vec![InjuryEffect::DisableHand],
    ));
    assert_eq!(ledger.hands_available(), HandsAvailable::new(0));
}

#[test]
fn disable_hand_on_non_arm_part_is_inert_for_hand_count() {
    // C3(e): a DisableHand carried by a non-arm injury (Torso here) is INERT for the hand
    // count — there is no hand to disable. Both hands remain.
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_on(BodyPart::Torso, vec![InjuryEffect::DisableHand]));
    assert_eq!(
        ledger.hands_available(),
        HandsAvailable::new(2),
        "a DisableHand on a non-arm part disables no hand"
    );
}

#[test]
fn disable_hand_effect_deserializes_from_ron() {
    // C1-adjacent: the fieldless DisableHand variant + a sibling Modify(Shooting) parse
    // from the same effects Vec — the .injury.ron shape the hand-disabling injuries use.
    let ron = r#"(
        name:         "Shattered Left Hand",
        category:     Arm,
        severity:     Major,
        popup_text:   "LEFT HAND SHATTERED",
        log_text:     "shatters a hand",
        inspect_text: "Shattered Left Hand",
        effects: [
            DisableHand,
            Modify(stat: Shooting, amount: -2),
        ],
    )"#;
    let parsed = ron::from_str::<InjuryDef>(ron);
    assert!(
        parsed.is_ok(),
        "the DisableHand + Modify effects Vec must deserialize: {parsed:?}"
    );
    let Ok(def) = parsed else {
        return;
    };
    assert_eq!(def.category, InjuryCategory::Arm);
    assert!(matches!(def.effects[0], InjuryEffect::DisableHand));
    assert!(matches!(
        def.effects[1],
        InjuryEffect::Modify {
            stat: StatTarget::Shooting,
            ..
        }
    ));
}
