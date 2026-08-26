use gdtf_battle_sim::{
    effects::attachments::{AimDelta, AttachmentEffect},
    terrain::facing::TerrainFacing,
};
use gdtf_content_families::sprites::{SpriteImagePath, SpriteSource};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{
    AttachmentEffectNet, EditorListIndexNet, EditorListMemberNet, EditorListNet, EditorListOpNet,
    SpriteSourceNet, TerrainFacingNet,
};

fn a_frame_source() -> SpriteSourceNet {
    SpriteSourceNet::from_source(&SpriteSource::File(SpriteImagePath::new(
        "sprites/frame.png".to_owned(),
    )))
}

#[test]
fn the_named_list_round_trips() {
    for list in [
        EditorListNet::EntrySides,
        EditorListNet::AttachmentEffects,
        EditorListNet::SpriteFrames,
    ] {
        assert_ron_round_trip(&list);
    }
}

#[test]
fn every_toggle_arm_round_trips() {
    for facing in TerrainFacing::ALL {
        assert_ron_round_trip(&EditorListOpNet::Toggle(TerrainFacingNet::from_facing(
            facing,
        )));
    }
}

#[test]
fn every_list_op_arm_round_trips() {
    let index = EditorListIndexNet::new(2);
    assert_ron_round_trip(&EditorListOpNet::Add);
    assert_ron_round_trip(&EditorListOpNet::Remove(index));
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
    assert_ron_round_trip(&EditorListMemberNet::AttachmentEffect(
        AttachmentEffectNet::from_effect(&AttachmentEffect::Aim(AimDelta::new(0.0))),
    ));
    assert_ron_round_trip(&EditorListMemberNet::SpriteFrame(a_frame_source()));
}

#[test]
fn the_list_types_trace_usable_shapes() {
    assert_schema_is_usable::<EditorListNet>("EditorListNet");
    assert_schema_is_usable::<EditorListOpNet>("EditorListOpNet");
    assert_schema_is_usable::<EditorListMemberNet>("EditorListMemberNet");
    assert_schema_is_usable::<EditorListIndexNet>("EditorListIndexNet");
}
