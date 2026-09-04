use gdtf_battle_sim::injuries::{InjuryEffect, StatDelta, StatTarget};

use super::super::assert_ron_round_trip;
use crate::mcp::wire::{
    EditorDraftNameNet, EditorFieldNet, EditorListIndexNet, InjuryCategoryNet, InjuryEffectNet,
    InjuryFieldNet, InjuryKeyNet, InjurySeverityNet, InjuryTextNet,
};

fn an_effect() -> InjuryEffectNet {
    InjuryEffectNet::from_effect(InjuryEffect::Modify {
        stat:   StatTarget::Speed,
        amount: StatDelta::new(-1),
    })
}

#[test]
fn every_injury_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::Injury(InjuryFieldNet::Key(
        InjuryKeyNet::new("cracked_rib"),
    )));
    assert_ron_round_trip(&EditorFieldNet::Injury(InjuryFieldNet::Name(
        EditorDraftNameNet::new("cracked rib"),
    )));
    assert_ron_round_trip(&EditorFieldNet::Injury(InjuryFieldNet::Category(
        InjuryCategoryNet::Torso,
    )));
    assert_ron_round_trip(&EditorFieldNet::Injury(InjuryFieldNet::Severity(
        InjurySeverityNet::Critical,
    )));
    assert_ron_round_trip(&EditorFieldNet::Injury(InjuryFieldNet::PopupText(
        InjuryTextNet::new("crack"),
    )));
    assert_ron_round_trip(&EditorFieldNet::Injury(InjuryFieldNet::LogText(
        InjuryTextNet::new("a crack"),
    )));
    assert_ron_round_trip(&EditorFieldNet::Injury(InjuryFieldNet::InspectText(
        InjuryTextNet::new("a rib gives way"),
    )));
    assert_ron_round_trip(&EditorFieldNet::Injury(InjuryFieldNet::Effect {
        index:  EditorListIndexNet::new(0),
        effect: an_effect(),
    }));
}
