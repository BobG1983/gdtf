use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::HeightBand,
    terrain::def::{LosBlocking, TerrainTag},
    weapon::WeaponName,
};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    net_qa::wire::{
        ArmorHardnessNet, ArmorProtectionNet, BlocksPathingNet, EditorDraftNameNet, EditorFieldNet,
        FootfallNet, HeightBandNet, LosBlockingNet, MountedWeaponNet, TerrainHpNet, TerrainTagNet,
        TileRoleNet,
    },
    terrain_form::{FootfallChoice, offered_graphic_roles},
};

#[test]
fn every_terrain_value_round_trips() {
    assert_ron_round_trip(&TerrainHpNet::new(40));
    for band in [HeightBand::Low, HeightBand::Mid, HeightBand::High] {
        assert_ron_round_trip(&HeightBandNet::from_band(band));
    }
    for choice in FootfallChoice::ALL {
        assert_ron_round_trip(&FootfallNet::from_choice(choice));
    }
    assert_ron_round_trip(&MountedWeaponNet::from_name(&WeaponName::new(
        "autogun".to_owned(),
    )));
    assert_ron_round_trip(&BlocksPathingNet::new(true));
    for blocking in [
        LosBlocking::Full,
        LosBlocking::UpToHeightBand,
        LosBlocking::None,
    ] {
        assert_ron_round_trip(&LosBlockingNet::from_blocking(blocking));
    }
    for tag in TerrainTagNet::ALL {
        assert_ron_round_trip(&tag);
    }
}

#[test]
fn every_terrain_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::TerrainDisplayName(
        EditorDraftNameNet::new("rusted bulkhead"),
    ));
    assert_ron_round_trip(&EditorFieldNet::TerrainHp(TerrainHpNet::new(40)));
    assert_ron_round_trip(&EditorFieldNet::TerrainArmorProtection(
        ArmorProtectionNet::from_protection(ArmorProtection::new(4)),
    ));
    assert_ron_round_trip(&EditorFieldNet::TerrainArmorHardness(
        ArmorHardnessNet::from_hardness(ArmorHardness::new(2)),
    ));
    assert_ron_round_trip(&EditorFieldNet::TerrainHeightBand(
        HeightBandNet::from_band(HeightBand::Mid),
    ));
    assert_ron_round_trip(&EditorFieldNet::TerrainGraphic(TileRoleNet::Slab));
    assert_ron_round_trip(&EditorFieldNet::TerrainFootfall(FootfallNet::Grate));
    assert_ron_round_trip(&EditorFieldNet::TerrainMountedWeapon(Some(
        MountedWeaponNet::from_name(&WeaponName::new("autogun".to_owned())),
    )));
    assert_ron_round_trip(&EditorFieldNet::TerrainMountedWeapon(None));
    assert_ron_round_trip(&EditorFieldNet::TerrainBlocksPathing(Some(
        BlocksPathingNet::new(false),
    )));
    assert_ron_round_trip(&EditorFieldNet::TerrainBlocksPathing(None));
    assert_ron_round_trip(&EditorFieldNet::TerrainBlocksLos(Some(
        LosBlockingNet::UpToHeightBand,
    )));
    assert_ron_round_trip(&EditorFieldNet::TerrainBlocksLos(None));
}

#[test]
fn the_graphic_arms_are_exactly_the_roles_the_form_offers() {
    let offered = offered_graphic_roles();
    for role in &offered {
        assert!(
            TileRoleNet::from_role(*role).is_some(),
            "{role:?} is a role the form's picker offers, so the wire needs an arm for it",
        );
    }
    for arm in TileRoleNet::ALL {
        assert!(
            offered.contains(&arm.to_role()),
            "{arm:?} is a wire arm the form's picker does not offer, so a client could send a \
             role no author can pick",
        );
    }
    assert_eq!(
        offered.len(),
        TileRoleNet::ALL.len(),
        "the picker and the wire hold the same number of roles",
    );
}

#[test]
fn every_terrain_tag_maps_back_to_the_sim() {
    for tag in [
        TerrainTag::Openable,
        TerrainTag::BlocksVision,
        TerrainTag::BlocksPathfinding,
        TerrainTag::Indestructible,
    ] {
        assert_eq!(
            TerrainTagNet::from_tag(tag).to_tag(),
            tag,
            "the wire tag reads back as the sim's own",
        );
    }
}

#[test]
fn the_terrain_values_trace_usable_shapes() {
    assert_schema_is_usable::<TerrainHpNet>("TerrainHpNet");
    assert_schema_is_usable::<HeightBandNet>("HeightBandNet");
    assert_schema_is_usable::<FootfallNet>("FootfallNet");
    assert_schema_is_usable::<MountedWeaponNet>("MountedWeaponNet");
    assert_schema_is_usable::<BlocksPathingNet>("BlocksPathingNet");
    assert_schema_is_usable::<LosBlockingNet>("LosBlockingNet");
    assert_schema_is_usable::<TerrainTagNet>("TerrainTagNet");
}
