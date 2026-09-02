use super::super::{OwedViews, TerrainView, TerrainViews, owed_views};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::{
        def::{
            LeavesBehind, TerrainDef, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind,
            TerrainTag, TerrainUuid,
        },
        facing::{TerrainCorner, TerrainFacing},
        piece::TerrainGraphicKey,
    },
};

fn def(sim_kind: TerrainSimKind, tags: Vec<TerrainTag>) -> TerrainDef {
    TerrainDef {
        key: TerrainUuid::generate(),
        display_name: TerrainDisplayName::new("Owed Probe".to_owned()),
        sim_kind,
        presenter_kind: TerrainPresenterKind::Cover {
            graphic_name: TerrainGraphicKey::new("cover".to_owned()),
        },
        views: TerrainViews::new(Vec::new()),
        tags,
        on_death: Vec::new(),
        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
    }
}

fn wall_kind() -> TerrainSimKind {
    TerrainSimKind::Wall {
        hp:               CoverHp::new(40),
        armor_protection: ArmorProtection::new(6),
        armor_hardness:   ArmorHardness::new(3),
        height_band:      HeightBand::High,
    }
}

fn cover_kind() -> TerrainSimKind {
    TerrainSimKind::Cover {
        hp:               CoverHp::new(20),
        armor_protection: ArmorProtection::new(3),
        armor_hardness:   ArmorHardness::new(1),
        height_band:      HeightBand::Low,
    }
}

fn slab_kind() -> TerrainSimKind {
    TerrainSimKind::Slab {
        hp:               SlabHp::new(120),
        armor_protection: ArmorProtection::new(4),
        armor_hardness:   ArmorHardness::new(2),
    }
}

fn door_set() -> Vec<TerrainView> {
    TerrainFacing::ALL
        .into_iter()
        .flat_map(|facing| [TerrainView::Shut(facing), TerrainView::Open(facing)])
        .collect()
}

fn stair_set() -> Vec<TerrainView> {
    TerrainFacing::ALL
        .into_iter()
        .flat_map(|facing| {
            [
                TerrainView::FromBelow(facing),
                TerrainView::FromAbove(facing),
            ]
        })
        .collect()
}

fn wall_set() -> Vec<TerrainView> {
    TerrainFacing::ALL
        .into_iter()
        .map(TerrainView::Edge)
        .chain(TerrainCorner::ALL.into_iter().map(TerrainView::Corner))
        .collect()
}

fn faced_set() -> Vec<TerrainView> {
    TerrainFacing::ALL
        .into_iter()
        .map(TerrainView::Facing)
        .collect()
}

// The views in `left` that `right` does not hold.
fn only_in(left: &[TerrainView], right: &OwedViews) -> Vec<TerrainView> {
    left.iter()
        .filter(|view| !right.contains(view))
        .copied()
        .collect()
}

fn assert_owes(def: &TerrainDef, expected: &[TerrainView], what: &str) {
    let owed = owed_views(def);
    let missing = only_in(expected, &owed);
    let unexpected: Vec<TerrainView> = owed
        .iter()
        .filter(|view| !expected.contains(view))
        .copied()
        .collect();
    assert!(
        missing.is_empty() && unexpected.is_empty(),
        "{what} owes exactly its own view set — missing {missing:?}, and owes {unexpected:?} that \
         it should not",
    );
}

#[test]
fn each_def_owes_the_view_set_its_marker_names() {
    assert_owes(
        &def(wall_kind(), vec![TerrainTag::Openable]),
        &door_set(),
        "a door, which authors sim_kind: Wall and the Openable tag,",
    );
    assert_owes(
        &def(slab_kind(), vec![TerrainTag::Stair]),
        &stair_set(),
        "a staircase, which authors sim_kind: Slab and the Stair tag,",
    );
    assert_owes(&def(wall_kind(), Vec::new()), &wall_set(), "a plain wall");
    assert_owes(
        &def(cover_kind(), Vec::new()),
        &faced_set(),
        "a cover piece",
    );
    assert_owes(
        &def(slab_kind(), Vec::new()),
        &[TerrainView::Single],
        "a plain floor slab",
    );
    assert_owes(
        &def(wall_kind(), vec![TerrainTag::Openable, TerrainTag::Stair]),
        &door_set(),
        "a def carrying both tags, which the Openable row claims first,",
    );
}
