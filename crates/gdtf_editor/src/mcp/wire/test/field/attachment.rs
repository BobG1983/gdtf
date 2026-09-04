use gdtf_battle_sim::{
    effects::attachments::{AimDelta, AttachmentEffect},
    equipment::attachments::AttachmentSlot,
};

use super::{super::assert_ron_round_trip, support::a_name};
use crate::mcp::wire::{
    AttachmentEffectNet, AttachmentFieldNet, AttachmentSlotNet, EditorFieldNet, EditorListIndexNet,
};

#[test]
fn every_attachment_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::Attachment(
        AttachmentFieldNet::DisplayName(a_name()),
    ));
    for slot in AttachmentSlot::ALL {
        assert_ron_round_trip(&EditorFieldNet::Attachment(AttachmentFieldNet::Slot(
            AttachmentSlotNet::from_slot(slot),
        )));
    }
    assert_ron_round_trip(&EditorFieldNet::Attachment(AttachmentFieldNet::Effect {
        index:  EditorListIndexNet::new(0),
        effect: AttachmentEffectNet::from_effect(&AttachmentEffect::Aim(AimDelta::new(0.5))),
    }));
}
