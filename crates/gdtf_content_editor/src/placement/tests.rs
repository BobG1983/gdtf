use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::{GridHeight, GridLevels, GridSize, GridWidth, ThemeUuid},
    metric::{CellLevel, Level},
    prelude::Cell,
    slab::SlabHp,
    terrain::{
        def::{
            LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid, TerrainViews,
        },
        facing::TerrainFacing,
        piece::TerrainGraphicKey,
    },
};

use super::{
    EditorTileClass, IllegalReason, PlacementVerdict, ProposedPlacement, apply_placement, classify,
    evaluate_placement,
};
use crate::editor_map::{EditorMap, PaintedPiece};

const NORTH: TerrainFacing = TerrainFacing::North;

fn theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0149_1490_0001))
}

const fn tu(n: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
}

fn size() -> GridSize {
    GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(3))
        .unwrap_or_else(|_| GridSize::default())
}

fn slab_def(key: TerrainUuid, label: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new(label.to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               SlabHp::new(50),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("slab".to_owned()),
            footfall:     None,
        },
        tags: Vec::new(),
        views: TerrainViews::new(Vec::new()),
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
    }
}

fn cover_def(key: TerrainUuid, label: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new(label.to_owned()),
        sim_kind: TerrainSimKind::Cover {
            hp:               CoverHp::new(20),
            armor_protection: ArmorProtection::new(2),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      HeightBand::Low,
        },
        presenter_kind: TerrainPresenterKind::Cover {
            graphic_name: TerrainGraphicKey::new("cover".to_owned()),
        },
        tags: Vec::new(),
        views: TerrainViews::new(Vec::new()),
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
    }
}

const FLOOR: TerrainUuid = tu(0x01);
const SLAB: TerrainUuid = tu(0x02);
const LADDER: TerrainUuid = tu(0x03);

fn registry() -> TerrainDefRegistry {
    TerrainDefRegistry::new([
        (FLOOR, cover_def(FLOOR, "Deck Floor")),
        (SLAB, slab_def(SLAB, "Deck Slab")),
        (LADDER, cover_def(LADDER, "Steel Ladder")),
    ])
}

fn ground(cell: Cell) -> CellLevel {
    CellLevel::new(cell, Level::new(0))
}

fn above(cell: Cell) -> CellLevel {
    CellLevel::new(cell, Level::new(1))
}

#[test]
fn classify_reads_kind_and_name() {
    let reg = registry();
    let th = theme();
    assert_eq!(
        classify(&reg, th, &SLAB),
        EditorTileClass::Slab,
        "a TerrainSimKind::Slab def classifies as a slab",
    );
    assert_eq!(
        classify(&reg, th, &LADDER),
        EditorTileClass::Ladder,
        "a ladder-named def classifies as a ladder (the model has no ladder kind)",
    );
    assert_eq!(
        classify(&reg, th, &FLOOR),
        EditorTileClass::Other,
        "a plain cover/floor def is unconstrained by the vertical rules",
    );
}

#[test]
fn legal_floor_placement_commits_and_mutates_map() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(1, 1);
    let placement = ProposedPlacement::new(ground(cell), FLOOR, TerrainFacing::East);

    assert_eq!(
        evaluate_placement(&map, &reg, th, &placement, size()),
        PlacementVerdict::legal(),
        "a non-slab on empty ground is plainly legal",
    );
    assert!(
        apply_placement(&mut map, &reg, th, &placement, size()),
        "a legal placement commits",
    );
    assert_eq!(
        map.tile_at(cell),
        Some(PaintedPiece::new(FLOOR, TerrainFacing::East)),
        "a committed placement carries the proposal's tile AND facing into the map",
    );
    assert_eq!(map.painted_count(), 1);
}

#[test]
fn slab_on_ladder_same_cell_is_illegal_and_rejected() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(2, 2);

    let ladder = ProposedPlacement::new(ground(cell), LADDER, NORTH);
    assert!(apply_placement(&mut map, &reg, th, &ladder, size()));
    assert_eq!(map.painted_count(), 1, "the ladder commits at L0");

    let slab = ProposedPlacement::new(ground(cell), SLAB, NORTH);
    assert_eq!(
        evaluate_placement(&map, &reg, th, &slab, size()),
        PlacementVerdict::Illegal(IllegalReason::SlabSealsLadder),
        "a slab onto a ladder seals it — illegal (C2)",
    );
    assert!(
        !apply_placement(&mut map, &reg, th, &slab, size()),
        "an illegal placement is rejected (C2)",
    );
    assert_eq!(
        map.painted_count(),
        1,
        "a rejected placement leaves the map UNCHANGED (C2)",
    );
    assert_eq!(
        map.tile_at(cell),
        Some(PaintedPiece::new(LADDER, NORTH)),
        "the ladder is untouched — the illegal slab never overwrote it (C2)",
    );
}

#[test]
fn slab_above_ladder_is_illegal_and_rejected() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(1, 3);

    let ladder = ProposedPlacement::new(ground(cell), LADDER, NORTH);
    assert!(apply_placement(&mut map, &reg, th, &ladder, size()));

    let slab = ProposedPlacement::new(above(cell), SLAB, NORTH);
    assert_eq!(
        evaluate_placement(&map, &reg, th, &slab, size()),
        PlacementVerdict::Illegal(IllegalReason::SlabSealsLadder),
        "a slab over a ladder seals its destination — illegal (C2)",
    );
    assert!(
        !apply_placement(&mut map, &reg, th, &slab, size()),
        "an illegal placement is rejected (C2)",
    );
    assert!(
        map.tile_at_level(above(cell)).is_none(),
        "the illegal slab never entered the map (C2)",
    );
    assert_eq!(map.painted_count(), 1, "only the ladder remains (C2)");
}

#[test]
fn ladder_auto_clears_slab_above() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(0, 0);

    assert!(map.paint_at(above(cell), SLAB, NORTH, size()));
    assert_eq!(map.painted_count(), 1, "the slab is seeded at L1");

    let ladder = ProposedPlacement::new(ground(cell), LADDER, NORTH);
    assert_eq!(
        evaluate_placement(&map, &reg, th, &ladder, size()),
        PlacementVerdict::legal_clearing(above(cell)),
        "a ladder under a slab is legal and auto-clears the slab above (C1)",
    );
    assert!(
        apply_placement(&mut map, &reg, th, &ladder, size()),
        "the ladder placement commits (C1)",
    );
    assert!(
        map.tile_at_level(above(cell)).is_none(),
        "the slab directly above the ladder was AUTO-CLEARED (C1)",
    );
    assert_eq!(
        map.tile_at(cell),
        Some(PaintedPiece::new(LADDER, NORTH)),
        "the ladder was painted at L0 (C1)",
    );
    assert_eq!(
        map.painted_count(),
        1,
        "the slab is gone and the ladder is present — one entry (C1)",
    );
}

#[test]
fn ladder_without_slab_above_is_plain_legal() {
    let reg = registry();
    let th = theme();
    let map = EditorMap::new();
    let cell = Cell::new(3, 3);
    let ladder = ProposedPlacement::new(ground(cell), LADDER, NORTH);
    assert_eq!(
        evaluate_placement(&map, &reg, th, &ladder, size()),
        PlacementVerdict::legal(),
        "a ladder with nothing above is plainly legal, no auto-clear",
    );
}

#[test]
fn out_of_bounds_is_illegal() {
    let reg = registry();
    let th = theme();
    let map = EditorMap::new();
    let placement = ProposedPlacement::new(ground(Cell::new(4, 0)), FLOOR, NORTH);
    assert_eq!(
        evaluate_placement(&map, &reg, th, &placement, size()),
        PlacementVerdict::Illegal(IllegalReason::OutOfBounds),
        "a slot off the grid is illegal (C3)",
    );
}
