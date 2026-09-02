//! Test terrain defs that author a full set of per-view art, one per view row a
//! resolver can select.

use super::{
    super::situation::test_pieces,
    defs::{display, graphic, view_row},
};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::{
        def::{
            LeavesBehind, TerrainDef, TerrainPresenterKind, TerrainSimKind, TerrainTag,
            TerrainView, TerrainViewArt, TerrainViews,
        },
        facing::{TerrainCorner, TerrainFacing},
    },
};

// The sprite a facing that no assertion reads names, so every owed row is filled.
const fn filler(facing: TerrainFacing) -> &'static str {
    match facing {
        TerrainFacing::North => "floor",
        TerrainFacing::East => "floor_alt_panel",
        TerrainFacing::South => "rubble",
        TerrainFacing::West => "slab_destroyed",
    }
}

// Shut and open art on all four sides; the three rows the door assertions read are distinct.
fn door_views() -> Vec<TerrainViewArt> {
    TerrainFacing::ALL
        .into_iter()
        .flat_map(|facing| {
            let shut = match facing {
                TerrainFacing::North => "door",
                other => filler(other),
            };
            let open = match facing {
                TerrainFacing::North => "door_ns",
                TerrainFacing::East => "door_ew",
                other => filler(other),
            };
            [
                view_row(TerrainView::Shut(facing), shut),
                view_row(TerrainView::Open(facing), open),
            ]
        })
        .collect()
}

// A distinct key per edge facing, and one corner key that is neither `wall` nor `wall_ew`.
fn facing_wall_views() -> Vec<TerrainViewArt> {
    let edges = TerrainFacing::ALL.into_iter().map(|facing| {
        let sprite = match facing {
            TerrainFacing::North => "wall",
            TerrainFacing::East => "wall_ew",
            TerrainFacing::South => "rubble",
            TerrainFacing::West => "floor_alt_panel",
        };
        view_row(TerrainView::Edge(facing), sprite)
    });
    let corners = TerrainCorner::ALL
        .into_iter()
        .map(|corner| view_row(TerrainView::Corner(corner), "slab"));
    edges.chain(corners).collect()
}

// The climb seen from below and from above on all four sides.
fn stair_views() -> Vec<TerrainViewArt> {
    TerrainFacing::ALL
        .into_iter()
        .flat_map(|facing| {
            let below = match facing {
                TerrainFacing::North => "stair_ns_up",
                other => filler(other),
            };
            let above = match facing {
                TerrainFacing::North => "stair_ns_down",
                other => filler(other),
            };
            [
                view_row(TerrainView::FromBelow(facing), below),
                view_row(TerrainView::FromAbove(facing), above),
            ]
        })
        .collect()
}

/// A `Wall`-kind def carrying [`TerrainTag::Openable`], with shut and open art per facing.
#[must_use]
pub(super) fn test_door() -> TerrainDef {
    TerrainDef {
        key:            test_pieces::DOOR,
        display_name:   display("Test Door"),
        sim_kind:       TerrainSimKind::Wall {
            hp:               CoverHp::new(40),
            armor_protection: ArmorProtection::new(2),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: graphic("door"),
        },
        tags:           vec![TerrainTag::Openable],
        views:          TerrainViews::new(door_views()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

/// A plain `Wall`-kind def whose four edge rows each name a different sprite key.
#[must_use]
pub(super) fn test_facing_wall() -> TerrainDef {
    TerrainDef {
        key:            test_pieces::FACING_WALL,
        display_name:   display("Test Facing Wall"),
        sim_kind:       TerrainSimKind::Wall {
            hp:               CoverHp::new(120),
            armor_protection: ArmorProtection::new(8),
            armor_hardness:   ArmorHardness::new(4),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: graphic("wall"),
        },
        tags:           Vec::new(),
        views:          TerrainViews::new(facing_wall_views()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

/// A `Slab`-kind def carrying [`TerrainTag::Stair`], with the climb art per facing.
#[must_use]
pub(super) fn test_stair() -> TerrainDef {
    TerrainDef {
        key:            test_pieces::STAIR,
        display_name:   display("Test Stair"),
        sim_kind:       TerrainSimKind::Slab {
            hp:               SlabHp::new(120),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: graphic("stair_ns_up"),
            footfall:     None,
        },
        tags:           vec![TerrainTag::Stair],
        views:          TerrainViews::new(stair_views()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}
