use bevy::ecs::world::World;

use super::*;
use crate::metric::{Cell, CellLevel, Level, MAX_LEVELS};

fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// The grid extent constants as `i32` cell coordinates — a checked conversion
/// (`GRID_WIDTH`/`GRID_HEIGHT` are `usize` so a raw `as i32` cast trips
/// `cast_possible_wrap`). The fallback is unreachable for the 60-cell extents but
/// keeps the test free of `unwrap`/`expect` (denied in tests too).
fn extent_i32(extent: usize) -> i32 {
    i32::try_from(extent).unwrap_or(i32::MAX)
}

/// C8(a) — `build_from_occupancy_input` from a HAND-BUILT input (entities
/// spawned in a real Bevy `World`, terrain placed) populates terrain + occupant
/// slots correctly, asserted CELL-BY-CELL.
///
/// Spawns two real entities, hand-builds an `OccupancyInput` with a wall, a
/// cover, and those two occupants at distinct `(cell, level)`s, builds the grid,
/// then walks every authored slot and asserts BOTH its terrain marker and its
/// occupant handle, plus that an untouched slot is `Open` / empty. The occupant
/// is the spawned `Entity` handle, never a numeric id (GTW-10 / GTW-12).
#[test]
fn build_from_input_populates_terrain_and_occupant_cell_by_cell() {
    // Real entities from a real World (the C8(a) requirement).
    let mut world = World::new();
    let alice = world.spawn_empty().id();
    let bob = world.spawn_empty().id();

    let wall_at = key(1, 2, 0);
    let cover_at = key(3, 4, 1);
    let alice_at = key(5, 6, 0);
    let bob_at = key(7, 8, 2);
    let empty_at = key(10, 10, 0);

    let input = OccupancyInput {
        terrain:   vec![
            TerrainPlacement::new(wall_at, TerrainKind::Wall),
            TerrainPlacement::new(cover_at, TerrainKind::Cover),
        ],
        occupants: vec![
            OccupantPlacement::new(alice_at, alice),
            OccupantPlacement::new(bob_at, bob),
        ],
    };

    let grid = OccupancyGrid::build_from_occupancy_input(&input);

    // Terrain, cell by cell.
    assert_eq!(
        grid.terrain(&wall_at),
        TerrainKind::Wall,
        "the wall slot must carry TerrainKind::Wall",
    );
    assert_eq!(
        grid.terrain(&cover_at),
        TerrainKind::Cover,
        "the cover slot must carry TerrainKind::Cover",
    );
    // Occupants, cell by cell — the exact spawned Entity handles.
    assert_eq!(
        grid.occupant(&alice_at),
        Some(alice),
        "alice's slot must hold alice's Entity handle",
    );
    assert_eq!(
        grid.occupant(&bob_at),
        Some(bob),
        "bob's slot must hold bob's Entity handle",
    );
    // A slot the situation never touched is Open with no occupant.
    assert_eq!(
        grid.terrain(&empty_at),
        TerrainKind::Open,
        "an untouched slot must default to Open",
    );
    assert_eq!(
        grid.occupant(&empty_at),
        None,
        "an untouched slot must have no occupant",
    );
    // A terrain slot carries no occupant and an occupant slot is Open terrain —
    // the two facts are independent per slot.
    assert_eq!(grid.occupant(&wall_at), None);
    assert_eq!(grid.terrain(&alice_at), TerrainKind::Open);
}

/// C8(b) — marking a cell destroyed EXCLUDES it from the blocking query.
///
/// A cover cell blocks while it stands; after `mark_cover_destroyed` it must
/// NOT block (C6), while an unrelated standing cover cell still blocks — proving
/// the exclusion is per-cell, not global.
#[test]
fn destroyed_cover_is_excluded_from_blocking() {
    let input = OccupancyInput {
        terrain:   vec![
            TerrainPlacement::new(key(2, 2, 0), TerrainKind::Cover),
            TerrainPlacement::new(key(9, 9, 0), TerrainKind::Cover),
        ],
        occupants: Vec::new(),
    };
    let mut grid = OccupancyGrid::build_from_occupancy_input(&input);

    let smashed = key(2, 2, 0);
    let intact = key(9, 9, 0);

    // Both cover cells block while standing.
    assert!(grid.is_blocked(&smashed), "standing cover must block");
    assert!(grid.is_blocked(&intact), "standing cover must block");

    // Mark one destroyed — it must no longer block, the other still blocks.
    grid.mark_cover_destroyed(smashed);
    assert!(
        !grid.is_blocked(&smashed),
        "a destroyed cover cell must NOT block (C6)",
    );
    assert!(
        grid.is_blocked(&intact),
        "an unrelated standing cover cell must still block",
    );
    // The destroyed-cover set records the smashed cell.
    assert!(grid.is_cover_destroyed(&smashed));
    assert!(!grid.is_cover_destroyed(&intact));
}

/// A wall blocks and Open does not — the static blocking-ness of the terrain
/// marker (independent of destruction).
#[test]
fn wall_blocks_open_does_not() {
    let input = OccupancyInput {
        terrain:   vec![TerrainPlacement::new(key(4, 4, 0), TerrainKind::Wall)],
        occupants: Vec::new(),
    };
    let grid = OccupancyGrid::build_from_occupancy_input(&input);

    assert!(grid.is_blocked(&key(4, 4, 0)), "a wall must block");
    assert!(
        !grid.is_blocked(&key(0, 0, 0)),
        "an Open cell must not block",
    );
    assert!(TerrainKind::Wall.blocks());
    assert!(TerrainKind::Cover.blocks());
    assert!(!TerrainKind::Open.blocks());
}

/// The destroyed-cover set is **append-only** and survives a rebuild only by
/// re-appending: a freshly built grid starts with an empty set, and the only way
/// to grow it is `mark_cover_destroyed` (there is no remove API).
#[test]
fn destroyed_cover_is_append_only() {
    let mut grid = OccupancyGrid::new();
    let a = key(1, 1, 0);
    let b = key(2, 2, 0);

    assert!(
        grid.destroyed_cover().is_empty(),
        "a fresh grid has no destroyed cover",
    );

    grid.mark_cover_destroyed(a);
    grid.mark_cover_destroyed(b);
    // Re-marking is a harmless no-op (set semantics) — still two cells.
    grid.mark_cover_destroyed(a);

    assert_eq!(grid.destroyed_cover().len(), 2, "two distinct cells marked");
    assert!(grid.destroyed_cover().contains(&a));
    assert!(grid.destroyed_cover().contains(&b));

    // A fresh build does NOT carry the set forward (occupancy is rebuilt fresh;
    // carrying destroyed cover across a rebuild is the caller's append — E1.7).
    let rebuilt = OccupancyGrid::build_from_occupancy_input(&OccupancyInput::new());
    assert!(
        rebuilt.destroyed_cover().is_empty(),
        "a rebuild starts with an empty destroyed-cover set",
    );
}

/// Out-of-range coordinates are handled gracefully — no panic, and they read as
/// Open / empty / not-blocked. Probes negative and past-extent coordinates on
/// every axis.
#[test]
fn out_of_range_coords_are_graceful() {
    let mut grid = OccupancyGrid::new();

    let negative = key(-1, 5, 0);
    let past_x = key(extent_i32(GRID_WIDTH), 0, 0);
    let past_y = key(0, extent_i32(GRID_HEIGHT), 0);
    let past_level = key(0, 0, MAX_LEVELS);

    for oob in [negative, past_x, past_y, past_level] {
        assert_eq!(grid.terrain(&oob), TerrainKind::Open);
        assert_eq!(grid.occupant(&oob), None);
        assert!(
            !grid.is_blocked(&oob),
            "an out-of-range cell must not block"
        );
        assert!(grid.slot(&oob).is_none());
        // Setting on an out-of-range key is a graceful no-op (no panic).
        grid.set_terrain(oob, TerrainKind::Wall);
        grid.set_occupant(oob, None);
        assert_eq!(grid.terrain(&oob), TerrainKind::Open);
    }
}

/// The grid spans the full 60×60×8 extent — the structural constants. The corner
/// `(59, 59, 7)` is in-range (last valid slot) and `(60, 60, 8)` is out — pinning
/// the STRUCTURAL dimensions (system definition, not balance tuning).
#[test]
fn grid_spans_full_extent() {
    assert_eq!(GRID_WIDTH, 60);
    assert_eq!(GRID_HEIGHT, 60);
    assert_eq!(MAX_LEVELS, 8);

    let grid = OccupancyGrid::new();
    let last = key(
        extent_i32(GRID_WIDTH) - 1,
        extent_i32(GRID_HEIGHT) - 1,
        MAX_LEVELS - 1,
    );
    assert!(
        grid.slot(&last).is_some(),
        "(59,59,7) is the last valid slot"
    );

    let past = key(extent_i32(GRID_WIDTH), extent_i32(GRID_HEIGHT), MAX_LEVELS);
    assert!(grid.slot(&past).is_none(), "(60,60,8) is out of range");
}

/// A later occupant placement at the same `(cell, level)` overwrites an earlier
/// one — the pour applies placements in order. (Two occupants on one cell is the
/// situation author's concern, E1.8; the grid just stores the last write.)
#[test]
fn later_occupant_placement_wins() {
    let mut world = World::new();
    let first = world.spawn_empty().id();
    let second = world.spawn_empty().id();
    let at = key(3, 3, 0);

    let input = OccupancyInput {
        terrain:   Vec::new(),
        occupants: vec![
            OccupantPlacement::new(at, first),
            OccupantPlacement::new(at, second),
        ],
    };
    let grid = OccupancyGrid::build_from_occupancy_input(&input);
    assert_eq!(
        grid.occupant(&at),
        Some(second),
        "the later placement at the same cell wins",
    );
}
