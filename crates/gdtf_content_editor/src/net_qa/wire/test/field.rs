use gdtf_battle_sim::{
    armor::{ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart},
    effects::attachments::{AimDelta, AttachmentEffect},
    equipment::attachments::AttachmentSlot,
};
use gdtf_content_families::sprites::{
    SpriteFps, SpriteImagePath, SpritePx, SpriteRect, SpriteSource,
};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    net_qa::wire::{
        ArmorFloorNet, ArmorHardnessNet, ArmorIntegrityNet, ArmorProtectionNet, ArmorTypeNet,
        AttachmentEffectNet, AttachmentSlotNet, BodyPartNet, EditorDraftNameNet, EditorFieldNet,
        EditorListIndexNet, SpriteAnimatedNet, SpriteFacingNet, SpriteFpsNet, SpritePxNet,
        SpriteSourceNet, TerrainKindNet,
    },
    terrain_form::TerrainKindChoice,
};

fn a_source() -> SpriteSourceNet {
    SpriteSourceNet::from_source(&SpriteSource::Sheet {
        sheet: SpriteImagePath::new("sprites/sheet.png".to_owned()),
        rect:  SpriteRect {
            x: SpritePx::new(0),
            y: SpritePx::new(0),
            w: SpritePx::new(16),
            h: SpritePx::new(16),
        },
    })
}

fn a_name() -> EditorDraftNameNet {
    EditorDraftNameNet::new("scarred plate")
}

#[test]
fn every_terrain_field_arm_round_trips() {
    for choice in TerrainKindChoice::SEGMENT_ORDER {
        assert_ron_round_trip(&EditorFieldNet::Kind(TerrainKindNet::from_choice(choice)));
    }
}

#[test]
fn every_name_field_arm_round_trips() {
    let name = EditorDraftNameNet::new("scarred plate");
    assert_ron_round_trip(&name);
    assert_ron_round_trip(&EditorFieldNet::ArmorName(a_name()));
    assert_ron_round_trip(&EditorFieldNet::SpriteName(a_name()));
    assert_ron_round_trip(&EditorFieldNet::AttachmentName(a_name()));
}

#[test]
fn every_armor_field_arm_round_trips_for_every_piece() {
    for part in BodyPart::ALL {
        let part = BodyPartNet::from_part(part);
        assert_ron_round_trip(&EditorFieldNet::ArmorFloor {
            part,
            value: ArmorFloorNet::from_floor(ArmorFloor::new(3)),
        });
        assert_ron_round_trip(&EditorFieldNet::ArmorProtection {
            part,
            value: ArmorProtectionNet::from_protection(ArmorProtection::new(4)),
        });
        assert_ron_round_trip(&EditorFieldNet::ArmorHardness {
            part,
            value: ArmorHardnessNet::from_hardness(ArmorHardness::new(5)),
        });
        assert_ron_round_trip(&EditorFieldNet::ArmorIntegrity {
            part,
            value: ArmorIntegrityNet::from_integrity(ArmorIntegrity::new(600)),
        });
        for armor_type in ArmorType::ALL {
            assert_ron_round_trip(&EditorFieldNet::ArmorType {
                part,
                value: ArmorTypeNet::from_type(armor_type),
            });
        }
    }
}

#[test]
fn every_sprite_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::SpriteBaseSource(a_source()));
    assert_ron_round_trip(&EditorFieldNet::SpriteAnchorX(SpritePxNet::from_px(
        SpritePx::new(2),
    )));
    assert_ron_round_trip(&EditorFieldNet::SpriteAnchorY(SpritePxNet::from_px(
        SpritePx::new(3),
    )));
    assert_ron_round_trip(&EditorFieldNet::SpriteFps(SpriteFpsNet::from_fps(
        SpriteFps::new(12.0),
    )));
    for facing in SpriteFacingNet::ALL {
        assert_ron_round_trip(&EditorFieldNet::SpriteFacingOverride {
            facing,
            source: Some(a_source()),
        });
        assert_ron_round_trip(&EditorFieldNet::SpriteFacingOverride {
            facing,
            source: None,
        });
    }
    assert_ron_round_trip(&EditorFieldNet::SpriteFrame {
        index:  EditorListIndexNet::new(1),
        source: a_source(),
    });
    assert_ron_round_trip(&EditorFieldNet::SpriteAnimated(SpriteAnimatedNet::new(
        true,
    )));
}

#[test]
fn every_attachment_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::AttachmentDisplayName(a_name()));
    for slot in AttachmentSlot::ALL {
        assert_ron_round_trip(&EditorFieldNet::AttachmentSlot(
            AttachmentSlotNet::from_slot(slot),
        ));
    }
    assert_ron_round_trip(&EditorFieldNet::AttachmentEffect {
        index:  EditorListIndexNet::new(0),
        effect: AttachmentEffectNet::from_effect(&AttachmentEffect::Aim(AimDelta::new(0.5))),
    });
}

#[test]
fn the_field_traces_a_usable_shape() {
    assert_schema_is_usable::<EditorFieldNet>("EditorFieldNet");
    assert_schema_is_usable::<EditorDraftNameNet>("EditorDraftNameNet");
}
