use super::{
    super::{
        BleedAfflicted, BleedAmount, InflictedInjuries, InjuryEffect, StatDelta, StatDeltaSum,
        StatKind, StatTarget,
    },
    support::*,
};

#[test]
fn stat_target_kind_splits_first_eight_attribute_rest_derived() {
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
    assert_eq!(ledger.delta_for(StatTarget::Hp), StatDeltaSum::default());
}

#[test]
fn gain_folds_a_multi_effect_injury_and_stacks_across_injuries() {
    let mut ledger = InflictedInjuries::default();

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

    ledger.gain(gained_with(vec![
        InjuryEffect::Modify {
            stat:   StatTarget::Aim,
            amount: StatDelta::new(-3),
        },
        InjuryEffect::Bleeding {
            amount: BleedAmount::new(1),
        },
    ]));

    assert_eq!(ledger.delta_for(StatTarget::Aim), StatDeltaSum::new(-5));
    assert_eq!(
        ledger.delta_for(StatTarget::Shooting),
        StatDeltaSum::new(-1)
    );
    assert_eq!(ledger.bleed(), BleedAfflicted::new(3));
    assert_eq!(ledger.gained().len(), 2);
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
