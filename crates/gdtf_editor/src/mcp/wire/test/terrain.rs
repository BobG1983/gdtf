use gdtf_battle_sim::{
    cover::HeightBand,
    terrain::{
        def::{LeavesBehind, LosBlocking, TerrainTag, TerrainUuid, TerrainView},
        facing::{TerrainCorner, TerrainFacing},
        piece::TerrainGraphicKey,
    },
    weapon::WeaponName,
};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    mcp::wire::{
        BlocksPathingNet, FootfallNet, HeightBandNet, LeavesBehindNet, LosBlockingNet,
        MountedWeaponNet, TerrainHpNet, TerrainTagNet, TerrainViewNet, TerrainViewSpriteNet,
    },
    terrain_form::FootfallChoice,
};

// Every arm `TerrainViewNet` mirrors, each facing and corner walked in ring order.
fn every_view_arm() -> Vec<TerrainViewNet> {
    let mut arms: Vec<TerrainViewNet> = Vec::new();
    for facing in TerrainFacing::ALL {
        for view in [
            TerrainView::Edge(facing),
            TerrainView::Facing(facing),
            TerrainView::Shut(facing),
            TerrainView::Open(facing),
            TerrainView::FromBelow(facing),
            TerrainView::FromAbove(facing),
        ] {
            arms.push(TerrainViewNet::from_view(view));
        }
    }
    for corner in TerrainCorner::ALL {
        arms.push(TerrainViewNet::from_view(TerrainView::Corner(corner)));
    }
    arms.push(TerrainViewNet::from_view(TerrainView::Single));
    arms
}

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
    for leaves in [
        LeavesBehind::Nothing,
        LeavesBehind::Piece(TerrainUuid::nil()),
        LeavesBehind::Sprite(TerrainGraphicKey::new("rubble".to_owned())),
    ] {
        assert_ron_round_trip(&LeavesBehindNet::from_leaves_behind(&leaves));
    }
    for view in every_view_arm() {
        assert_ron_round_trip(&view);
    }
    assert_ron_round_trip(&TerrainViewSpriteNet::from_key(&TerrainGraphicKey::new(
        "wall_ew".to_owned(),
    )));
}

#[test]
fn every_terrain_view_maps_back_to_the_sim() {
    for arm in every_view_arm() {
        assert_eq!(
            TerrainViewNet::from_view(arm.to_view()),
            arm,
            "the wire view reads back as the sim's own, or a write would land on another view",
        );
    }
}

#[test]
fn every_terrain_tag_maps_back_to_the_sim() {
    for tag in [
        TerrainTag::Openable,
        TerrainTag::Stair,
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
    assert_schema_is_usable::<LeavesBehindNet>("LeavesBehindNet");
    assert_schema_is_usable::<TerrainViewNet>("TerrainViewNet");
    assert_schema_is_usable::<TerrainViewSpriteNet>("TerrainViewSpriteNet");
}
