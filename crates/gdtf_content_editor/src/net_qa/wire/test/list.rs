use gdtf_battle_sim::{
    effects::attachments::{AimDelta, AttachmentEffect},
    equipment::attachments::{AttachmentName, AttachmentSlot, SlotCapacity},
    injuries::InjuryEffect,
    terrain::facing::TerrainFacing,
};
use gdtf_content_families::sprites::{SpriteImagePath, SpriteSource};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{
    AttachmentEffectNet, AttachmentKeyNet, AttachmentSlotNet, EditorDraftNameNet,
    EditorListIndexNet, EditorListMemberNet, EditorListNet, EditorListOpNet, FightModeKindNet,
    FightModeSpecNet, InjuryEffectNet, SlotCapacityNet, SpriteSourceNet, StrikesNet,
    TerrainFacingNet, TerrainTagNet, TuCostNet, WeaponSlotNet,
};

fn a_frame_source() -> SpriteSourceNet {
    SpriteSourceNet::from_source(&SpriteSource::File(SpriteImagePath::new(
        "sprites/frame.png".to_owned(),
    )))
}

fn a_fight_mode() -> FightModeSpecNet {
    FightModeSpecNet::new(
        FightModeKindNet::Swing,
        TuCostNet::new(0),
        StrikesNet::new(1),
    )
}

fn a_slot() -> WeaponSlotNet {
    WeaponSlotNet::new(
        AttachmentSlotNet::from_slot(AttachmentSlot::Muzzle),
        SlotCapacityNet::new(*SlotCapacity::new(1)),
    )
}

#[test]
fn the_named_list_round_trips() {
    for list in [
        EditorListNet::EntrySides,
        EditorListNet::TerrainTags,
        EditorListNet::AttachmentEffects,
        EditorListNet::SpriteFrames,
        EditorListNet::InjuryEffects,
        EditorListNet::MeleeWeaponFightModes,
        EditorListNet::MeleeWeaponSlots,
        EditorListNet::MeleeWeaponAttachments,
        EditorListNet::GangMembers,
    ] {
        assert_ron_round_trip(&list);
    }
}

#[test]
fn every_toggle_arm_round_trips() {
    for facing in TerrainFacing::ALL {
        assert_ron_round_trip(&EditorListOpNet::Toggle(EditorListMemberNet::EntrySide(
            TerrainFacingNet::from_facing(facing),
        )));
    }
    for tag in TerrainTagNet::ALL {
        assert_ron_round_trip(&EditorListOpNet::Toggle(EditorListMemberNet::TerrainTag(
            tag,
        )));
    }
}

#[test]
fn every_list_op_arm_round_trips() {
    let index = EditorListIndexNet::new(2);
    assert_ron_round_trip(&EditorListOpNet::Add);
    assert_ron_round_trip(&EditorListOpNet::Remove(index));
    assert_ron_round_trip(&EditorListOpNet::SetAt(
        index,
        EditorListMemberNet::FightMode(a_fight_mode()),
    ));
    assert_ron_round_trip(&EditorListOpNet::MoveUp(index));
    assert_ron_round_trip(&EditorListOpNet::MoveDown(index));
}

#[test]
fn every_member_round_trips() {
    for facing in TerrainFacing::ALL {
        assert_ron_round_trip(&EditorListMemberNet::EntrySide(
            TerrainFacingNet::from_facing(facing),
        ));
    }
    for tag in TerrainTagNet::ALL {
        assert_ron_round_trip(&EditorListMemberNet::TerrainTag(tag));
    }
    assert_ron_round_trip(&EditorListMemberNet::AttachmentEffect(
        AttachmentEffectNet::from_effect(&AttachmentEffect::Aim(AimDelta::new(0.0))),
    ));
    assert_ron_round_trip(&EditorListMemberNet::SpriteFrame(a_frame_source()));
    assert_ron_round_trip(&EditorListMemberNet::InjuryEffect(
        InjuryEffectNet::from_effect(InjuryEffect::DisableHand),
    ));
    assert_ron_round_trip(&EditorListMemberNet::FightMode(a_fight_mode()));
    assert_ron_round_trip(&EditorListMemberNet::Slot(a_slot()));
    assert_ron_round_trip(&EditorListMemberNet::Attachment(
        AttachmentKeyNet::from_key(&AttachmentName::new("chain_teeth".to_owned())),
    ));
    assert_ron_round_trip(&EditorListMemberNet::GangMember(EditorDraftNameNet::new(
        "Kez",
    )));
}

#[test]
fn the_list_types_trace_usable_shapes() {
    assert_schema_is_usable::<EditorListNet>("EditorListNet");
    assert_schema_is_usable::<EditorListOpNet>("EditorListOpNet");
    assert_schema_is_usable::<EditorListMemberNet>("EditorListMemberNet");
    assert_schema_is_usable::<EditorListIndexNet>("EditorListIndexNet");
}
