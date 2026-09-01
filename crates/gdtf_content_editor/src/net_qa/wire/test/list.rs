use gdtf_battle_sim::{
    armor::ArmorType,
    effects::{
        attachments::{AimDelta, AttachmentEffect},
        fields::FieldKey,
        on_death::{ExplodeDamage, OnDeathEffect},
    },
    equipment::attachments::{AttachmentName, AttachmentSlot, SlotCapacity},
    injuries::{InjuryEffect, InjuryName, InjuryWeight, WeightedInjuryEntry},
    terrain::facing::TerrainFacing,
    weapon::{
        AoeRange, BlastRadius, DamageType, FireModeSpec, HitType, ModeConeMult, ModeKind,
        ModeShots, ModeTuPercent,
    },
};
use gdtf_content_families::sprites::{SpriteImagePath, SpriteSource};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{
    ArmorTypeNet, AttachmentEffectNet, AttachmentKeyNet, AttachmentSlotNet, EditorDraftNameNet,
    EditorListIndexNet, EditorListMemberNet, EditorListNet, EditorListOpNet, FightModeKindNet,
    FightModeSpecNet, FireModeSpecNet, InjuryEffectNet, OnDeathEffectNet, SlotCapacityNet,
    SpriteSourceNet, StrikesNet, TerrainFacingNet, TerrainTagNet, TuCostNet, WeaponSlotNet,
    WeightingBucketNet, WeightingRowNet,
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

fn a_fire_mode() -> FireModeSpecNet {
    FireModeSpecNet::from_spec(FireModeSpec::with_hit_type(
        ModeKind::Burst,
        ModeConeMult::new(1.5),
        ModeTuPercent::new(0.4),
        ModeShots::new(3),
        HitType::Line {
            range: AoeRange::new(4),
        },
    ))
}

fn a_weighting_row() -> WeightingRowNet {
    WeightingRowNet::from_entry(&WeightedInjuryEntry::new(
        InjuryName::new("twisted_ankle".to_owned()),
        InjuryWeight::new(5),
    ))
}

fn an_explode() -> OnDeathEffect {
    OnDeathEffect::Explode {
        hit_type:    HitType::Blast {
            radius: BlastRadius::new(2),
        },
        damage:      ExplodeDamage::new(12),
        damage_type: DamageType::Blast,
    }
}

fn a_leave_field() -> OnDeathEffect {
    OnDeathEffect::LeaveField {
        field: FieldKey::new("promethium_pool".to_owned()),
    }
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
        EditorListNet::TerrainOnDeathEffects,
        EditorListNet::WeaponOnDeathEffects,
        EditorListNet::AttachmentEffects,
        EditorListNet::SpriteFrames,
        EditorListNet::InjuryEffects,
        EditorListNet::MeleeWeaponFightModes,
        EditorListNet::MeleeWeaponSlots,
        EditorListNet::MeleeWeaponAttachments,
        EditorListNet::GangMembers,
        EditorListNet::WeaponFireModes,
        EditorListNet::WeaponSlots,
        EditorListNet::WeaponAttachments,
        EditorListNet::FieldImmuneArmorTypes,
        EditorListNet::WeightingBucket(WeightingBucketNet::Minor),
        EditorListNet::WeightingBucket(WeightingBucketNet::Major),
        EditorListNet::WeightingBucket(WeightingBucketNet::Critical),
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
    for armor_type in ArmorType::ALL {
        assert_ron_round_trip(&EditorListOpNet::Toggle(
            EditorListMemberNet::ImmuneArmorType(ArmorTypeNet::from_type(armor_type)),
        ));
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
    assert_ron_round_trip(&EditorListMemberNet::FireMode(a_fire_mode()));
    for armor_type in ArmorType::ALL {
        assert_ron_round_trip(&EditorListMemberNet::ImmuneArmorType(
            ArmorTypeNet::from_type(armor_type),
        ));
    }
    assert_ron_round_trip(&EditorListMemberNet::WeightingRow(a_weighting_row()));
    for effect in [an_explode(), a_leave_field()] {
        assert_ron_round_trip(&EditorListMemberNet::OnDeathEffect(
            OnDeathEffectNet::from_effect(&effect),
        ));
    }
}

#[test]
fn an_on_death_member_reads_back_as_the_sims_own_effect() {
    for effect in [an_explode(), a_leave_field()] {
        assert_eq!(
            OnDeathEffectNet::from_effect(&effect).to_effect(),
            effect,
            "the mirror carries the whole payload, so a client's effect reads back verbatim",
        );
    }
}

#[test]
fn the_list_types_trace_usable_shapes() {
    assert_schema_is_usable::<EditorListNet>("EditorListNet");
    assert_schema_is_usable::<EditorListOpNet>("EditorListOpNet");
    assert_schema_is_usable::<EditorListMemberNet>("EditorListMemberNet");
    assert_schema_is_usable::<EditorListIndexNet>("EditorListIndexNet");
    assert_schema_is_usable::<OnDeathEffectNet>("OnDeathEffectNet");
}
