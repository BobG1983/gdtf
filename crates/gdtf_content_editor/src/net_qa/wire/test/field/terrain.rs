use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::HeightBand,
    weapon::WeaponName,
};

use super::super::assert_ron_round_trip;
use crate::{
    net_qa::wire::{
        ArmorHardnessNet, ArmorProtectionNet, BlocksPathingNet, EditorDraftNameNet, EditorFieldNet,
        FootfallNet, HeightBandNet, LosBlockingNet, MountedWeaponNet, TerrainFieldNet,
        TerrainHpNet, TerrainKindNet, TileRoleNet,
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
}
