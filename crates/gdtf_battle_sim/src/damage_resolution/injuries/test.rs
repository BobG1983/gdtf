//! Vocabulary tests for the GTW-435 injury types — the [`InflictedInjuries::gain`]
//! ledger folding (C3) and the `.injury.ron` deserialize (C4).
//!
//! These exercise the REAL public API on the real types (no stubs): `gain` folds
//! through the actual [`StatDeltaLedger`] / [`BleedAfflicted`] accumulators, and the
//! RON test parses through the actual `serde` derive. Per the loader-tests rule, the
//! RON test asserts STRUCTURE (kinds resolve, the shape round-trips), never specific
//! tunable magnitudes.

use super::{
    BleedAfflicted, BleedAmount, GainedInjury, InflictedInjuries, InjuryDef, InjuryEffect,
    InjuryName, InspectText, PostHeal, StatDelta, StatDeltaSum, StatKind, StatTarget,
};
use crate::{armor::BodyPart, severity::Severity};

/// Build a minimal [`GainedInjury`] carrying the given effects (the texts and the
/// part/severity are irrelevant to the folding under test).
fn gained_with(effects: Vec<InjuryEffect>) -> GainedInjury {
    GainedInjury::new(
        InjuryName::new("test_injury".to_owned()),
        BodyPart::Head,
        Severity::Minor,
        effects,
        InspectText::new("a test injury".to_owned()),
    )
}

#[test]
fn stat_target_kind_splits_first_eight_attribute_rest_derived() {
    // The first eight ALL entries are attributes, the last eight are derived.
    for (i, stat) in StatTarget::ALL.into_iter().enumerate() {
        let expected = if i < 8 {
            StatKind::Attribute
        } else {
            StatKind::Derived
        };
        assert_eq!(
            stat.kind(),
            expected,
            "stat at index {i} has the wrong kind"
        );
        assert_eq!(stat.index(), i, "stat index must match its ALL position");
    }
    assert_eq!(StatTarget::COUNT, 16);
}

#[test]
fn gain_folds_a_modify_on_a_direct_attribute() {
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_with(vec![InjuryEffect::Modify {
        stat:   StatTarget::Aim,
        amount: StatDelta::new(-2),
    }]));

    assert_eq!(ledger.delta_for(StatTarget::Aim), StatDeltaSum::new(-2));
    // Unmentioned stats stay zero.
    assert_eq!(ledger.delta_for(StatTarget::Cool), StatDeltaSum::default());
    assert_eq!(ledger.gained().len(), 1);
}

#[test]
fn gain_folds_a_modify_on_a_derived_stat() {
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_with(vec![InjuryEffect::Modify {
        stat:   StatTarget::Shooting,
        amount: StatDelta::new(-1),
    }]));

    assert_eq!(StatTarget::Shooting.kind(), StatKind::Derived);
    assert_eq!(
        ledger.delta_for(StatTarget::Shooting),
        StatDeltaSum::new(-1)
    );
}

#[test]
fn gain_folds_a_bleeding_effect_into_the_bleed_accrual() {
    let mut ledger = InflictedInjuries::default();
    ledger.gain(gained_with(vec![InjuryEffect::Bleeding {
        amount: BleedAmount::new(3),
    }]));

    assert_eq!(ledger.bleed(), BleedAfflicted::new(3));
    // A Bleeding effect contributes no stat delta.
    assert_eq!(ledger.delta_for(StatTarget::Hp), StatDeltaSum::default());
}

#[test]
fn gain_folds_a_multi_effect_injury_and_stacks_across_injuries() {
    let mut ledger = InflictedInjuries::default();

    // A single multi-effect injury: two Modifies (one attribute, one derived) plus a
    // Bleeding — all fold from ONE gain().
    ledger.gain(gained_with(vec![
        InjuryEffect::Modify {
            stat:   StatTarget::Aim,
            amount: StatDelta::new(-2),
        },
        InjuryEffect::Modify {
            stat:   StatTarget::Shooting,
            amount: StatDelta::new(-1),
        },
        InjuryEffect::Bleeding {
            amount: BleedAmount::new(2),
        },
    ]));

    assert_eq!(ledger.delta_for(StatTarget::Aim), StatDeltaSum::new(-2));
    assert_eq!(
        ledger.delta_for(StatTarget::Shooting),
        StatDeltaSum::new(-1)
    );
    assert_eq!(ledger.bleed(), BleedAfflicted::new(2));
    assert_eq!(ledger.gained().len(), 1);

    // A second injury stacking on the SAME stat ADDS, and a second bleed ACCRUES.
    ledger.gain(gained_with(vec![
        InjuryEffect::Modify {
            stat:   StatTarget::Aim,
            amount: StatDelta::new(-3),
        },
        InjuryEffect::Bleeding {
            amount: BleedAmount::new(1),
        },
    ]));

    assert_eq!(ledger.delta_for(StatTarget::Aim), StatDeltaSum::new(-5)); // -2 + -3
    assert_eq!(
        ledger.delta_for(StatTarget::Shooting),
        StatDeltaSum::new(-1)
    ); // untouched
    assert_eq!(ledger.bleed(), BleedAfflicted::new(3)); // 2 + 1
    assert_eq!(ledger.gained().len(), 2); // ordered ledger grows
}

#[test]
fn gain_appends_to_the_ledger_in_infliction_order() {
    let mut ledger = InflictedInjuries::default();
    let first = gained_with(vec![InjuryEffect::Modify {
        stat:   StatTarget::Grit,
        amount: StatDelta::new(-1),
    }]);
    let second = gained_with(vec![InjuryEffect::Bleeding {
        amount: BleedAmount::new(1),
    }]);

    ledger.gain(first.clone());
    ledger.gain(second.clone());

    assert_eq!(ledger.gained(), &[first, second]);
}

#[test]
fn injury_def_deserializes_from_the_schema_ron() {
    // The design-doc schema sample. Structure-only assertions (loader-tests rule:
    // no pinned tunable magnitudes — assert the SHAPE, not specific weights/amounts).
    let ron = r#"(
        name:         "Lost Eye",
        body_part:    Head,
        severity:     Critical,
        popup_text:   "LOST EYE",
        log_text:     "loses an eye",
        inspect_text: "Lost Eye -- -2 Aim, -1 Cool",
        effects: [
            Modify(stat: Aim,  amount: -2),
            Modify(stat: Cool, amount: -1),
        ],
        post_heal:    Deferred,
    )"#;

    let parsed = ron::from_str::<InjuryDef>(ron);
    assert!(
        parsed.is_ok(),
        "schema RON must deserialize into InjuryDef: {parsed:?}"
    );
    let Ok(def) = parsed else {
        return;
    };

    assert_eq!(def.body_part, BodyPart::Head);
    assert_eq!(def.severity, Severity::Critical);
    assert_eq!(def.post_heal, PostHeal::Deferred);
    assert_eq!(def.effects.len(), 2);
    // Both effects parsed as Modify on the named stats (structure, not magnitude).
    assert!(matches!(
        def.effects[0],
        InjuryEffect::Modify {
            stat: StatTarget::Aim,
            ..
        }
    ));
    assert!(matches!(
        def.effects[1],
        InjuryEffect::Modify {
            stat: StatTarget::Cool,
            ..
        }
    ));
}

#[test]
fn injury_def_post_heal_defaults_to_deferred_when_omitted() {
    // post_heal is parsed-but-unread (C4): a floor file may omit it entirely.
    let ron = r#"(
        name:         "Scalp Graze",
        body_part:    Head,
        severity:     Minor,
        popup_text:   "SCALP GRAZE",
        log_text:     "is grazed across the scalp",
        inspect_text: "Scalp Graze",
        effects: [ Bleeding(amount: 1) ],
    )"#;

    let parsed = ron::from_str::<InjuryDef>(ron);
    assert!(
        parsed.is_ok(),
        "RON without post_heal must deserialize: {parsed:?}"
    );
    let Ok(def) = parsed else {
        return;
    };
    assert_eq!(def.post_heal, PostHeal::Deferred);
    assert!(matches!(def.effects[0], InjuryEffect::Bleeding { .. }));
}
