//! In-crate tests for the placement-legality rules (GTW-430): the semantics-driven
//! classification, the C1 ladder-auto-clear, the C2 slab-seals-ladder rejection, and the C3
//! out-of-bounds verdict.

use gdtf_battle_sim::{
    Cell,
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::{GridHeight, GridLevels, GridSize, GridWidth, ThemeUuid},
    metric::{CellLevel, Level},
    slab::SlabHp,
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid,
        },
        piece::TerrainGraphicKey,
    },
};

use super::{
    EditorTileClass, IllegalReason, PlacementVerdict, ProposedPlacement, apply_placement, classify,
    evaluate_placement,
};
use crate::editor_map::EditorMap;

/// The theme key the test registry is associated with (a UUID; the registry resolves a
/// terrain by UUID regardless of theme).
fn theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0149_1490_0001))
}

/// A terrain UUID built from a small constant (the test registry's keys).
const fn tu(n: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
}

/// A `4 × 4 × 3` drawable volume — enough storeys for the L0/L1 vertical rules, with a
/// `1 × 1 × 1` fallback (the constructor is fallible; the fallback keeps the test panic-free).
fn size() -> GridSize {
    GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(3))
        .unwrap_or_else(|_| GridSize::default())
}

/// A SLAB terrain def (the magnitudes are throwaway DATA — not pinned).
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
        on_death: None,
    }
}

/// A COVER terrain def (a tile the vertical rules never constrain).
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
        on_death: None,
    }
}

/// A registry with a cover tile (`floor`), a slab tile (`slab`), and a ladder-named tile
/// (`ladder`, recognised by display name). The ladder tile's `sim_kind` is plain Cover so the
/// classifier's name path is what flags it.
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

/// Classification is semantics-driven: slab from the sim KIND, ladder from the NAME,
/// everything else Other.
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

/// A plain legal placement: painting a non-slab on the ground plane commits and mutates the
/// map.
#[test]
fn legal_floor_placement_commits_and_mutates_map() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(1, 1);
    let placement = ProposedPlacement::new(ground(cell), FLOOR);

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
        Some(FLOOR),
        "a committed placement mutates the map (legal case)",
    );
    assert_eq!(map.painted_count(), 1);
}

/// C2: a slab onto a cell that already holds a ladder (SAME slot) is ILLEGAL — rejected, map
/// UNCHANGED. The case the single-plane (`L0`) canvas reaches.
///
/// Pin-discriminating: with the rule reverted (a slab onto a ladder treated as legal) the
/// verdict would be `Legal`, `apply_placement` would return `true`, and the ladder would be
/// OVERWRITTEN by the slab — every assertion here would flip.
#[test]
fn slab_on_ladder_same_cell_is_illegal_and_rejected() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(2, 2);

    // Place a ladder on the ground (L0). Recognised by display name; a plain legal placement.
    let ladder = ProposedPlacement::new(ground(cell), LADDER);
    assert!(apply_placement(&mut map, &reg, th, &ladder, size()));
    assert_eq!(map.painted_count(), 1, "the ladder commits at L0");

    // Now try to slab the SAME cell (L0) — illegal (a slab can't seal a ladder's shaft).
    let slab = ProposedPlacement::new(ground(cell), SLAB);
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
        Some(LADDER),
        "the ladder is untouched — the illegal slab never overwrote it (C2)",
    );
}

/// C2 (multi-level): a slab directly ABOVE an existing ladder is ILLEGAL — rejected, map
/// UNCHANGED (the ladder's destination would be sealed).
#[test]
fn slab_above_ladder_is_illegal_and_rejected() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(1, 3);

    let ladder = ProposedPlacement::new(ground(cell), LADDER);
    assert!(apply_placement(&mut map, &reg, th, &ladder, size()));

    let slab = ProposedPlacement::new(above(cell), SLAB);
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

/// C1: placing a LADDER auto-clears a SLAB directly above it — legal, with the slab removed.
///
/// Pin-discriminating: with the C1 rule reverted (no auto-clear) the verdict would be a plain
/// `Legal { auto_clear: None }`, the slab above would SURVIVE, and the painted count would be 2.
#[test]
fn ladder_auto_clears_slab_above() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(0, 0);

    // Seed a slab one storey UP (L1) directly, as if previously painted.
    assert!(map.paint_at(above(cell), SLAB, size()));
    assert_eq!(map.painted_count(), 1, "the slab is seeded at L1");

    // Now place a ladder below it (L0): legal, and it auto-clears the slab above (C1).
    let ladder = ProposedPlacement::new(ground(cell), LADDER);
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
        Some(LADDER),
        "the ladder was painted at L0 (C1)",
    );
    assert_eq!(
        map.painted_count(),
        1,
        "the slab is gone and the ladder is present — one entry (C1)",
    );
}

/// A ladder with NO slab above is a plain legal placement (no spurious auto-clear).
#[test]
fn ladder_without_slab_above_is_plain_legal() {
    let reg = registry();
    let th = theme();
    let map = EditorMap::new();
    let cell = Cell::new(3, 3);
    let ladder = ProposedPlacement::new(ground(cell), LADDER);
    assert_eq!(
        evaluate_placement(&map, &reg, th, &ladder, size()),
        PlacementVerdict::legal(),
        "a ladder with nothing above is plainly legal, no auto-clear",
    );
}

/// An out-of-bounds slot is illegal through the shared predicate (C3 surfaced as a verdict).
#[test]
fn out_of_bounds_is_illegal() {
    let reg = registry();
    let th = theme();
    let map = EditorMap::new();
    // x = 4 is past the 4-wide extent (valid 0..4).
    let placement = ProposedPlacement::new(ground(Cell::new(4, 0)), FLOOR);
    assert_eq!(
        evaluate_placement(&map, &reg, th, &placement, size()),
        PlacementVerdict::Illegal(IllegalReason::OutOfBounds),
        "a slot off the grid is illegal (C3)",
    );
}
