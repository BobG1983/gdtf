use super::{super::assert_ron_round_trip, support::a_name};
use crate::net_qa::wire::{
    EditorFieldNet, EditorKeyNet, EditorListIndexNet, GangAttributeNet, GangAttributeValueNet,
    GangFieldNet,
};

fn a_key() -> EditorKeyNet {
    EditorKeyNet::new("stub_gun".to_owned())
}

#[test]
fn every_gang_field_arm_round_trips() {
    let index = EditorListIndexNet::new(2);
    assert_ron_round_trip(&EditorFieldNet::Gang(GangFieldNet::Name(a_name())));
    assert_ron_round_trip(&EditorFieldNet::Gang(GangFieldNet::MemberName {
        index,
        name: a_name(),
    }));
    for attribute in GangAttributeNet::ALL {
        assert_ron_round_trip(&attribute);
        assert_ron_round_trip(&EditorFieldNet::Gang(GangFieldNet::MemberAttribute {
            index,
            attribute,
            value: GangAttributeValueNet::new(42.5),
        }));
    }
    assert_ron_round_trip(&GangAttributeValueNet::new(7.25));
    assert_ron_round_trip(&EditorFieldNet::Gang(GangFieldNet::MemberWeapon {
        index,
        key: a_key(),
    }));
    assert_ron_round_trip(&EditorFieldNet::Gang(GangFieldNet::MemberArmor {
        index,
        key: a_key(),
    }));
    assert_ron_round_trip(&EditorFieldNet::Gang(GangFieldNet::MemberMeleeWeapon {
        index,
        key: Some(a_key()),
    }));
    assert_ron_round_trip(&EditorFieldNet::Gang(GangFieldNet::MemberMeleeWeapon {
        index,
        key: None,
    }));
}
