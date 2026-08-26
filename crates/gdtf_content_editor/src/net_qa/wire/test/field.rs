use gdtf_battle_sim::{
    armor::{ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart},
    effects::{
        attachments::{AimDelta, AttachmentEffect},
        fields::FieldKey,
    },
    equipment::attachments::AttachmentSlot,
    weapon::{BlastRadius, DamageType, Handedness, HitType, TrajectoryStyle},
};
use gdtf_content_families::sprites::{
    SpriteFps, SpriteImagePath, SpritePx, SpriteRect, SpriteSource,
};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    net_qa::wire::{
        AccuracyNet, ArmorFloorNet, ArmorHardnessNet, ArmorIntegrityNet, ArmorProtectionNet,
        ArmorTypeNet, AttachmentEffectNet, AttachmentSlotNet, BaseSpreadNet, BodyPartNet,
        DamageTypeNet, DotDamageNet, DotEnabledNet, DotTurnsNet, EditorDraftNameNet,
        EditorFieldNet, EditorKeyNet, EditorListIndexNet, ExplodeDamageNet, FatalBiasNet,
        FieldKeyNet, GangAttributeNet, GangAttributeValueNet, HandednessNet, HitTypeNet,
        KickbackNet, MagazineSizeNet, OnDeathEnabledNet, OnDeathVariantNet, ReloadTuNet, ShoveNet,
        SpriteAnimatedNet, SpriteFacingNet, SpriteFpsNet, SpritePxNet, SpriteSourceNet, StableNet,
        TerrainKindNet, TrajectoryStyleNet, WeaponDamageNet, WeaponPunchNet, WeaponShredNet,
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

fn a_key() -> EditorKeyNet {
    EditorKeyNet::new("stub_gun".to_owned())
}

#[test]
fn every_terrain_field_arm_round_trips() {
    for choice in TerrainKindChoice::SEGMENT_ORDER {
        assert_ron_round_trip(&EditorFieldNet::TerrainKind(TerrainKindNet::from_choice(
            choice,
        )));
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

#[test]
fn every_weapon_stat_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::WeaponName(a_name()));
    assert_ron_round_trip(&EditorFieldNet::WeaponBaseSpread(BaseSpreadNet::new(0.25)));
    assert_ron_round_trip(&EditorFieldNet::WeaponAccuracy(AccuracyNet::new(0.75)));
    assert_ron_round_trip(&EditorFieldNet::WeaponKickback(KickbackNet::new(0.5)));
    assert_ron_round_trip(&EditorFieldNet::WeaponDamage(WeaponDamageNet::new(9)));
    assert_ron_round_trip(&EditorFieldNet::WeaponPunch(WeaponPunchNet::new(2)));
    assert_ron_round_trip(&EditorFieldNet::WeaponShred(WeaponShredNet::new(1)));
    assert_ron_round_trip(&EditorFieldNet::WeaponFatalBias(FatalBiasNet::new(0.4)));
    for damage_type in DamageType::ALL {
        assert_ron_round_trip(&EditorFieldNet::WeaponDamageType(
            DamageTypeNet::from_damage_type(damage_type),
        ));
    }
    for handedness in [Handedness::OneHanded, Handedness::TwoHanded] {
        assert_ron_round_trip(&EditorFieldNet::WeaponHandedness(
            HandednessNet::from_handedness(handedness),
        ));
    }
}

#[test]
fn every_weapon_handling_field_arm_round_trips() {
    for style in [TrajectoryStyle::Straight, TrajectoryStyle::Arc] {
        assert_ron_round_trip(&EditorFieldNet::WeaponTrajectory(
            TrajectoryStyleNet::from_style(style),
        ));
    }
    assert_ron_round_trip(&EditorFieldNet::WeaponStable(StableNet::new(true)));
    assert_ron_round_trip(&EditorFieldNet::WeaponShove(ShoveNet::new(false)));
    assert_ron_round_trip(&EditorFieldNet::WeaponMagazineSize(MagazineSizeNet::new(
        24,
    )));
    assert_ron_round_trip(&EditorFieldNet::WeaponMagazineReloadTu(ReloadTuNet::new(6)));
}

#[test]
fn every_weapon_dot_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::WeaponDot(DotEnabledNet::new(true)));
    assert_ron_round_trip(&EditorFieldNet::WeaponDotDamage(DotDamageNet::new(4)));
    assert_ron_round_trip(&EditorFieldNet::WeaponDotTurns(DotTurnsNet::new(3)));
    assert_ron_round_trip(&EditorFieldNet::WeaponDotDamageType(
        DamageTypeNet::from_damage_type(DamageType::Plasma),
    ));
}

#[test]
fn every_weapon_on_death_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::WeaponOnDeath(OnDeathEnabledNet::new(true)));
    for variant in [OnDeathVariantNet::Explode, OnDeathVariantNet::LeaveField] {
        assert_ron_round_trip(&EditorFieldNet::WeaponOnDeathVariant(variant));
    }
    assert_ron_round_trip(&EditorFieldNet::WeaponOnDeathHitType(
        HitTypeNet::from_hit_type(HitType::Blast {
            radius: BlastRadius::new(2),
        }),
    ));
    assert_ron_round_trip(&EditorFieldNet::WeaponOnDeathDamage(ExplodeDamageNet::new(
        12,
    )));
    assert_ron_round_trip(&EditorFieldNet::WeaponOnDeathDamageType(
        DamageTypeNet::from_damage_type(DamageType::Blast),
    ));
    assert_ron_round_trip(&EditorFieldNet::WeaponOnDeathField(FieldKeyNet::from_key(
        &FieldKey::new("promethium_pool".to_owned()),
    )));
}

#[test]
fn the_field_traces_a_usable_shape() {
    assert_schema_is_usable::<EditorFieldNet>("EditorFieldNet");
    assert_schema_is_usable::<EditorDraftNameNet>("EditorDraftNameNet");
    assert_schema_is_usable::<GangAttributeNet>("GangAttributeNet");
    assert_schema_is_usable::<GangAttributeValueNet>("GangAttributeValueNet");
}

#[test]
fn the_weapon_values_trace_usable_shapes() {
    assert_schema_is_usable::<BaseSpreadNet>("BaseSpreadNet");
    assert_schema_is_usable::<AccuracyNet>("AccuracyNet");
    assert_schema_is_usable::<KickbackNet>("KickbackNet");
    assert_schema_is_usable::<TrajectoryStyleNet>("TrajectoryStyleNet");
    assert_schema_is_usable::<StableNet>("StableNet");
    assert_schema_is_usable::<MagazineSizeNet>("MagazineSizeNet");
    assert_schema_is_usable::<ReloadTuNet>("ReloadTuNet");
    assert_schema_is_usable::<DotEnabledNet>("DotEnabledNet");
    assert_schema_is_usable::<DotDamageNet>("DotDamageNet");
    assert_schema_is_usable::<DotTurnsNet>("DotTurnsNet");
    assert_schema_is_usable::<OnDeathEnabledNet>("OnDeathEnabledNet");
    assert_schema_is_usable::<OnDeathVariantNet>("OnDeathVariantNet");
    assert_schema_is_usable::<ExplodeDamageNet>("ExplodeDamageNet");
    assert_schema_is_usable::<FieldKeyNet>("FieldKeyNet");
}
