use super::{super::assert_ron_round_trip, support::a_name};
use crate::net_qa::wire::{EditorDraftNameNet, EditorFieldNet};

#[test]
fn every_name_field_arm_round_trips() {
    let name = EditorDraftNameNet::new("scarred plate");
    assert_ron_round_trip(&name);
    assert_ron_round_trip(&EditorFieldNet::ArmorName(a_name()));
    assert_ron_round_trip(&EditorFieldNet::SpriteName(a_name()));
    assert_ron_round_trip(&EditorFieldNet::AttachmentName(a_name()));
}
