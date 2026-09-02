use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::HeightBand,
    effects::fields::FieldKey,
    terrain::{
        def::TerrainView,
        facing::{TerrainCorner, TerrainFacing},
        piece::TerrainGraphicKey,
    },
    weapon::{BlastRadius, DamageType, HitType, WeaponName},
};

use super::super::assert_ron_round_trip;
use crate::{
    net_qa::wire::{
        ArmorHardnessNet, ArmorProtectionNet, BlocksPathingNet, DamageTypeNet, EditorDraftNameNet,
        EditorFieldNet, EditorListIndexNet, ExplodeDamageNet, FieldKeyNet, FootfallNet,
        HeightBandNet, HitTypeNet, LosBlockingNet, MountedWeaponNet, OnDeathVariantNet,
        TerrainFieldNet, TerrainHpNet, TerrainKindNet, TerrainViewNet, TerrainViewSpriteNet,
        TileRoleNet,
    },
    terrain_form::TerrainKindChoice,
};

// One Terrain field arm, under the form that owns it.
fn terrain(field: TerrainFieldNet) -> EditorFieldNet {
    EditorFieldNet::Terrain(field)
}

#[test]
fn every_terrain_field_arm_round_trips() {
    for choice in TerrainKindChoice::SEGMENT_ORDER {
        assert_ron_round_trip(&terrain(TerrainFieldNet::Kind(
            TerrainKindNet::from_choice(choice),
        )));
    }
    assert_ron_round_trip(&terrain(TerrainFieldNet::DisplayName(
        EditorDraftNameNet::new("rusted bulkhead"),
    )));
    assert_ron_round_trip(&terrain(TerrainFieldNet::Hp(TerrainHpNet::new(40))));
    assert_ron_round_trip(&terrain(TerrainFieldNet::ArmorProtection(
        ArmorProtectionNet::from_protection(ArmorProtection::new(4)),
    )));
    assert_ron_round_trip(&terrain(TerrainFieldNet::ArmorHardness(
        ArmorHardnessNet::from_hardness(ArmorHardness::new(2)),
    )));
    assert_ron_round_trip(&terrain(TerrainFieldNet::HeightBand(
        HeightBandNet::from_band(HeightBand::Mid),
    )));
    assert_ron_round_trip(&terrain(TerrainFieldNet::Graphic(TileRoleNet::Slab)));
    assert_ron_round_trip(&terrain(TerrainFieldNet::Footfall(FootfallNet::Grate)));
    assert_ron_round_trip(&terrain(TerrainFieldNet::MountedWeapon(Some(
        MountedWeaponNet::from_name(&WeaponName::new("autogun".to_owned())),
    ))));
    assert_ron_round_trip(&terrain(TerrainFieldNet::MountedWeapon(None)));
    assert_ron_round_trip(&terrain(TerrainFieldNet::BlocksPathing(Some(
        BlocksPathingNet::new(false),
    ))));
    assert_ron_round_trip(&terrain(TerrainFieldNet::BlocksPathing(None)));
    assert_ron_round_trip(&terrain(TerrainFieldNet::BlocksLos(Some(
        LosBlockingNet::UpToHeightBand,
    ))));
    assert_ron_round_trip(&terrain(TerrainFieldNet::BlocksLos(None)));
    for view in [
        TerrainView::Edge(TerrainFacing::North),
        TerrainView::Corner(TerrainCorner::SouthWest),
        TerrainView::Facing(TerrainFacing::East),
        TerrainView::Shut(TerrainFacing::South),
        TerrainView::Open(TerrainFacing::West),
        TerrainView::FromBelow(TerrainFacing::North),
        TerrainView::FromAbove(TerrainFacing::East),
        TerrainView::Single,
    ] {
        assert_ron_round_trip(&terrain(TerrainFieldNet::View {
            view:   TerrainViewNet::from_view(view),
            sprite: TerrainViewSpriteNet::from_key(&TerrainGraphicKey::new("wall".to_owned())),
        }));
    }
    let index = EditorListIndexNet::new(1);
    for variant in [OnDeathVariantNet::Explode, OnDeathVariantNet::LeaveField] {
        assert_ron_round_trip(&terrain(TerrainFieldNet::OnDeathVariant { index, variant }));
    }
    assert_ron_round_trip(&terrain(TerrainFieldNet::OnDeathHitType {
        index,
        hit_type: HitTypeNet::from_hit_type(HitType::Blast {
            radius: BlastRadius::new(2),
        }),
    }));
    assert_ron_round_trip(&terrain(TerrainFieldNet::OnDeathDamage {
        index,
        damage: ExplodeDamageNet::new(12),
    }));
    assert_ron_round_trip(&terrain(TerrainFieldNet::OnDeathDamageType {
        index,
        damage_type: DamageTypeNet::from_damage_type(DamageType::Blast),
    }));
    assert_ron_round_trip(&terrain(TerrainFieldNet::OnDeathField {
        index,
        field: FieldKeyNet::from_key(&FieldKey::new("toxic_waste_pool".to_owned())),
    }));
}
