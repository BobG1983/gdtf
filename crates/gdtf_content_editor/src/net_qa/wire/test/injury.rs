use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{BleedAmount, InjuryEffect, MovementCostFactor, StatDelta, StatTarget},
    severity::Severity,
};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{
    BleedAmountNet, EditorDraftNameNet, EditorFieldNet, EditorListIndexNet, InjuryCategoryNet,
    InjuryEffectNet, InjuryKeyNet, InjurySeverityNet, InjuryTextNet, MovementCostFactorNet,
    StatDeltaNet,
};

fn an_effect() -> InjuryEffectNet {
    InjuryEffectNet::from_effect(InjuryEffect::Modify {
        stat:   StatTarget::Speed,
        amount: StatDelta::new(-1),
    })
}

#[test]
fn every_injury_value_round_trips() {
    assert_ron_round_trip(&InjuryKeyNet::new("cracked_rib"));
    assert_ron_round_trip(&InjuryTextNet::new("a rib gives way"));
    for category in InjuryCategory::ALL {
        assert_ron_round_trip(&InjuryCategoryNet::from_category(category));
    }
    for severity in [Severity::Minor, Severity::Major, Severity::Critical] {
        let Some(rank) = InjurySeverityNet::from_severity(severity) else {
            unreachable!("{severity:?} is one of the three the injury table offers");
        };
        assert_ron_round_trip(&rank);
    }
    assert_ron_round_trip(&StatDeltaNet::new(-1));
    assert_ron_round_trip(&BleedAmountNet::new(2));
    assert_ron_round_trip(&MovementCostFactorNet::new(1.25));
    for effect in [
        an_effect(),
        InjuryEffectNet::from_effect(InjuryEffect::Bleeding {
            amount: BleedAmount::new(3),
        }),
        InjuryEffectNet::from_effect(InjuryEffect::DisableHand),
        InjuryEffectNet::from_effect(InjuryEffect::MovementCostMul(MovementCostFactor::new(1.5))),
    ] {
        assert_ron_round_trip(&effect);
    }
}

#[test]
fn every_injury_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::InjuryKey(InjuryKeyNet::new("cracked_rib")));
    assert_ron_round_trip(&EditorFieldNet::InjuryName(EditorDraftNameNet::new(
        "cracked rib",
    )));
    assert_ron_round_trip(&EditorFieldNet::InjuryCategory(InjuryCategoryNet::Torso));
    assert_ron_round_trip(&EditorFieldNet::InjurySeverity(InjurySeverityNet::Critical));
    assert_ron_round_trip(&EditorFieldNet::InjuryPopupText(InjuryTextNet::new(
        "crack",
    )));
    assert_ron_round_trip(&EditorFieldNet::InjuryLogText(InjuryTextNet::new(
        "a crack",
    )));
    assert_ron_round_trip(&EditorFieldNet::InjuryInspectText(InjuryTextNet::new(
        "a rib gives way",
    )));
    assert_ron_round_trip(&EditorFieldNet::InjuryEffect {
        index:  EditorListIndexNet::new(0),
        effect: an_effect(),
    });
}

#[test]
fn every_severity_the_form_offers_reads_back_as_the_sims_own() {
    for severity in [Severity::Minor, Severity::Major, Severity::Critical] {
        let Some(rank) = InjurySeverityNet::from_severity(severity) else {
            unreachable!("{severity:?} is one of the three the injury table offers");
        };
        assert_eq!(rank.to_severity(), severity);
    }
    for absent in [Severity::None, Severity::Fatal] {
        assert!(
            InjurySeverityNet::from_severity(absent).is_none(),
            "{absent:?} is not tabled, so the wire spells no arm for it",
        );
    }
}

#[test]
fn the_injury_values_trace_usable_shapes() {
    assert_schema_is_usable::<InjuryKeyNet>("InjuryKeyNet");
    assert_schema_is_usable::<InjuryTextNet>("InjuryTextNet");
    assert_schema_is_usable::<InjuryCategoryNet>("InjuryCategoryNet");
    assert_schema_is_usable::<InjurySeverityNet>("InjurySeverityNet");
    assert_schema_is_usable::<StatDeltaNet>("StatDeltaNet");
    assert_schema_is_usable::<BleedAmountNet>("BleedAmountNet");
    assert_schema_is_usable::<MovementCostFactorNet>("MovementCostFactorNet");
    assert_schema_is_usable::<InjuryEffectNet>("InjuryEffectNet");
}
