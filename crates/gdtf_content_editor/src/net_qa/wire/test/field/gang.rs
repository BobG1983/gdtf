use super::{super::assert_ron_round_trip, support::a_name};
use crate::net_qa::wire::{
    EditorFieldNet, EditorKeyNet, EditorListIndexNet, GangAttributeNet, GangAttributeValueNet,
};

fn a_key() -> EditorKeyNet {
    EditorKeyNet::new("stub_gun".to_owned())
}

#[test]
fn every_gang_field_arm_round_trips() {
    let index = EditorListIndexNet::new(2);
    assert_ron_round_trip(&EditorFieldNet::GangName(a_name()));
    assert_ron_round_trip(&EditorFieldNet::GangMemberName {
        index,
        name: a_name(),
    });
    for attribute in GangAttributeNet::ALL {
        assert_ron_round_trip(&attribute);
        assert_ron_round_trip(&EditorFieldNet::GangMemberAttribute {
            index,
            attribute,
            value: GangAttributeValueNet::new(42.5),
        });
    }
    assert_ron_round_trip(&GangAttributeValueNet::new(7.25));
    assert_ron_round_trip(&EditorFieldNet::GangMemberWeapon {
        index,
        key: a_key(),
    });
    assert_ron_round_trip(&EditorFieldNet::GangMemberArmor {
        index,
        key: a_key(),
    });
    assert_ron_round_trip(&EditorFieldNet::GangMemberMeleeWeapon {
        index,
        key: Some(a_key()),
    });
    assert_ron_round_trip(&EditorFieldNet::GangMemberMeleeWeapon { index, key: None });
}
