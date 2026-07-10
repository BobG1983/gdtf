//! Blocking semantics + the destroyed-cover exclusion set (C6/E1.7).

use super::{
    super::{OccupancyGrid, OccupancyInput, TerrainKind, TerrainPlacement},
    support::*,
};

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
    let mut grid = OccupancyGrid::build_from_occupancy_input(&input, &no_stair_cells());

    let smashed = key(2, 2, 0);
    let intact = key(9, 9, 0);

    // Both cover cells block while standing.
    assert!(*grid.is_blocked(&smashed), "standing cover must block");
    assert!(*grid.is_blocked(&intact), "standing cover must block");

    // Mark one destroyed — it must no longer block, the other still blocks.
    grid.mark_cover_destroyed(smashed);
    assert!(
        !*grid.is_blocked(&smashed),
        "a destroyed cover cell must NOT block (C6)",
    );
    assert!(
        *grid.is_blocked(&intact),
        "an unrelated standing cover cell must still block",
    );
    // The destroyed-cover set records the smashed cell.
    assert!(*grid.is_cover_destroyed(&smashed));
    assert!(!*grid.is_cover_destroyed(&intact));
}

/// A wall blocks and Open does not — the static blocking-ness of the terrain
/// marker (independent of destruction).
#[test]
fn wall_blocks_open_does_not() {
    let input = OccupancyInput {
        terrain:   vec![TerrainPlacement::new(key(4, 4, 0), TerrainKind::Wall)],
        occupants: Vec::new(),
    };
    let grid = OccupancyGrid::build_from_occupancy_input(&input, &no_stair_cells());

    assert!(*grid.is_blocked(&key(4, 4, 0)), "a wall must block");
    assert!(
        !*grid.is_blocked(&key(0, 0, 0)),
        "an Open cell must not block",
    );
    assert!(*TerrainKind::Wall.blocks());
    assert!(*TerrainKind::Cover.blocks());
    assert!(!*TerrainKind::Open.blocks());
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
    let rebuilt =
        OccupancyGrid::build_from_occupancy_input(&OccupancyInput::new(), &no_stair_cells());
    assert!(
        rebuilt.destroyed_cover().is_empty(),
        "a rebuild starts with an empty destroyed-cover set",
    );
}
