use super::{super::assert_ron_round_trip, support::a_name};
use crate::net_qa::wire::{
    ArmorFieldNet, AttachmentFieldNet, EditorDraftNameNet, EditorFieldNet, SpriteFieldNet,
};

#[test]
fn every_name_field_arm_round_trips() {
    let name = EditorDraftNameNet::new("scarred plate");
    assert_ron_round_trip(&name);
    assert_ron_round_trip(&EditorFieldNet::Armor(ArmorFieldNet::Name(a_name())));
    assert_ron_round_trip(&EditorFieldNet::Sprite(SpriteFieldNet::Name(a_name())));
    assert_ron_round_trip(&EditorFieldNet::Attachment(AttachmentFieldNet::Name(
        a_name(),
    )));
}
