use bevy::ecs::world::World;

use super::*;
use crate::{
    cover::HeightBand,
    ganger::Tu,
    metric::{Cell, CellLevel, Level, MAX_LEVELS},
    tuning::{MoveCost, MoveCosts},
};

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
            OccupantPlacement::new(alice_at, alice, HeightBand::High),
            OccupantPlacement::new(bob_at, bob, HeightBand::Low),
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
    // The silhouette band is poured TOGETHER with the occupant (GTW-304): a placed
    // occupant must carry its band so the march can strike it.
    assert_eq!(
        grid.occupant_band(&alice_at),
        Some(HeightBand::High),
        "alice's slot must carry her placement band (poured with the occupant)",
    );
    assert_eq!(
        grid.occupant_band(&bob_at),
        Some(HeightBand::Low),
        "bob's slot must carry his placement band (poured with the occupant)",
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
    assert_eq!(
        grid.occupant_band(&empty_at),
        None,
        "an untouched slot must have no published band",
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
            OccupantPlacement::new(at, first, HeightBand::Low),
            OccupantPlacement::new(at, second, HeightBand::High),
        ],
    };
    let grid = OccupancyGrid::build_from_occupancy_input(&input);
    assert_eq!(
        grid.occupant(&at),
        Some(second),
        "the later placement at the same cell wins",
    );
    assert_eq!(
        grid.occupant_band(&at),
        Some(HeightBand::High),
        "the winning placement's band wins too (occupant + band poured together)",
    );
}

// ---------------------------------------------------------------------------
// GTW-12b — pathable_neighbors (E7, ADR-0005): same-storey 8-connected planar
// neighbour enumeration. Tests are RELATIONS-ONLY (C6): no pinned shipped
// magnitudes — costs are asserted by their DERIVATION over arbitrary terrain
// costs, never the `MoveCosts::default()` numbers.
// ---------------------------------------------------------------------------

/// Build a fresh grid with the given `(cell, level, terrain)` placements set —
/// a HAND-BUILT fixture (C6), NOT the real asset loader. Every other slot stays
/// the default [`TerrainKind::Open`].
fn grid_with(terrain: &[(CellLevel, TerrainKind)]) -> OccupancyGrid {
    let mut grid = OccupancyGrid::new();
    for &(at, kind) in terrain {
        grid.set_terrain(at, kind);
    }
    grid
}

/// Collect `pathable_neighbors` into a `Vec` of `(cell, cost)` for assertions,
/// reducing each [`CellLevel`] to its `(x, y, level)` triple so the relations
/// read clearly.
fn neighbours_of(
    origin: CellLevel,
    grid: &OccupancyGrid,
    costs: MoveCosts,
) -> Vec<((i32, i32, i32), Tu)> {
    pathable_neighbors(origin, grid, &costs)
        .map(|(cell, cost)| ((cell.x, cell.y, cell.z), cost))
        .collect()
}

/// Whether `cell` appears among the enumerated neighbours of `origin`.
fn yields_neighbour(
    origin: CellLevel,
    target: CellLevel,
    grid: &OccupancyGrid,
    costs: MoveCosts,
) -> bool {
    pathable_neighbors(origin, grid, &costs).any(|(cell, _)| cell == target)
}

/// C2/C4 — on an entirely OPEN grid an interior cell has all EIGHT 8-connected
/// planar neighbours (orthogonal + diagonal), all on the SAME storey, and none
/// blocked.
#[test]
fn open_interior_cell_has_all_eight_planar_neighbours() {
    let grid = OccupancyGrid::new(); // all Open
    let costs = MoveCosts::default();
    let origin = key(5, 5, 0);

    let cells: Vec<(i32, i32, i32)> = neighbours_of(origin, &grid, costs)
        .into_iter()
        .map(|(cell, _)| cell)
        .collect();

    assert_eq!(
        cells.len(),
        8,
        "an open interior cell has 8 planar neighbours"
    );
    // All eight surrounding cells, all on the origin's storey (dz = 0).
    let expected = [
        (4, 4, 0),
        (5, 4, 0),
        (6, 4, 0),
        (4, 5, 0),
        (6, 5, 0),
        (4, 6, 0),
        (5, 6, 0),
        (6, 6, 0),
    ];
    for want in expected {
        assert!(cells.contains(&want), "expected neighbour {want:?}");
    }
    // The origin is never its own neighbour, and nothing crosses a storey.
    assert!(
        !cells.contains(&(5, 5, 0)),
        "a cell is not its own neighbour"
    );
    assert!(
        cells.iter().all(|(_, _, z)| *z == origin.z),
        "every planar neighbour shares the origin's storey",
    );
}

/// C4 — a BLOCKED neighbour (standing wall / cover) is EXCLUDED, while an OPEN
/// neighbour is INCLUDED. Proven by toggling one orthogonal neighbour to Wall.
#[test]
fn blocked_neighbour_excluded_open_included() {
    let blocked = key(6, 5, 0); // due east of the origin
    let grid = grid_with(&[(blocked, TerrainKind::Wall)]);
    let costs = MoveCosts::default();
    let origin = key(5, 5, 0);

    assert!(
        !yields_neighbour(origin, blocked, &grid, costs),
        "a standing wall neighbour must be excluded (C4)",
    );
    assert!(
        yields_neighbour(origin, key(4, 5, 0), &grid, costs),
        "an open neighbour must be included (C4)",
    );
}

/// C4 — a DESTROYED-cover neighbour is WALKABLE (included), reusing the grid's
/// `is_blocked` destroyed-cover exclusion. A STANDING cover neighbour is
/// excluded; once marked destroyed it appears.
#[test]
fn destroyed_cover_neighbour_is_walkable() {
    let cover = key(6, 5, 0);
    let mut grid = grid_with(&[(cover, TerrainKind::Cover)]);
    let costs = MoveCosts::default();
    let origin = key(5, 5, 0);

    // Standing cover blocks → excluded.
    assert!(
        !yields_neighbour(origin, cover, &grid, costs),
        "a standing cover neighbour must be excluded (C4)",
    );
    // Destroyed cover is walkable → included.
    grid.mark_cover_destroyed(cover);
    assert!(
        yields_neighbour(origin, cover, &grid, costs),
        "a destroyed-cover neighbour must be walkable (C4)",
    );
}

/// C4 — an OUT-OF-BOUNDS planar offset is EXCLUDED. An origin in the corner
/// `(0, 0, 0)` has its three off-grid offsets (negative x / negative y) dropped,
/// leaving only the three in-bounds neighbours.
#[test]
fn out_of_bounds_neighbours_excluded() {
    let grid = OccupancyGrid::new();
    let costs = MoveCosts::default();
    let origin = key(0, 0, 0);

    let cells: Vec<(i32, i32, i32)> = neighbours_of(origin, &grid, costs)
        .into_iter()
        .map(|(cell, _)| cell)
        .collect();

    // Only the three in-grid cells survive; the five off-grid offsets are dropped.
    assert_eq!(
        cells.len(),
        3,
        "a corner cell has only 3 in-bounds neighbours"
    );
    assert!(cells.contains(&(1, 0, 0)));
    assert!(cells.contains(&(0, 1, 0)));
    assert!(cells.contains(&(1, 1, 0)));
    // No negative coordinate ever surfaces.
    assert!(
        cells.iter().all(|(x, y, _)| *x >= 0 && *y >= 0),
        "no out-of-bounds (negative) neighbour is yielded",
    );
}

/// C3 — a diagonal is PRESENT when at least ONE of its two shared-edge
/// orthogonal neighbours is walkable. Block ONE side; the diagonal survives.
#[test]
fn diagonal_present_when_one_shared_edge_walkable() {
    // Diagonal NE of the origin is (6, 6, 0); its shared-edge orthogonals are
    // (6, 5, 0) and (5, 6, 0). Block only ONE of them.
    let grid = grid_with(&[(key(6, 5, 0), TerrainKind::Wall)]);
    let costs = MoveCosts::default();
    let origin = key(5, 5, 0);
    let diagonal = key(6, 6, 0);

    assert!(
        yields_neighbour(origin, diagonal, &grid, costs),
        "a diagonal must survive when one shared-edge orthogonal is walkable (C3)",
    );
}

/// C3 — a diagonal is ABSENT (no corner-cutting) when BOTH its shared-edge
/// orthogonal neighbours are blocked, EVEN though the diagonal cell ITSELF is
/// open. Block both sides; the open diagonal must still be excluded.
#[test]
fn diagonal_absent_when_both_shared_edges_blocked() {
    // Diagonal NE (6, 6, 0) is OPEN; block BOTH its shared-edge orthogonals.
    let grid = grid_with(&[
        (key(6, 5, 0), TerrainKind::Wall),
        (key(5, 6, 0), TerrainKind::Wall),
    ]);
    let costs = MoveCosts::default();
    let origin = key(5, 5, 0);
    let diagonal = key(6, 6, 0);

    // The diagonal cell itself is walkable …
    assert!(
        !grid.is_blocked(&diagonal),
        "the diagonal cell itself is open"
    );
    // … yet the step is illegal — both shared-edge orthogonals are blocked (C3).
    assert!(
        !yields_neighbour(origin, diagonal, &grid, costs),
        "no corner-cutting: a diagonal between two blocked cells is excluded (C3)",
    );
}

/// C2 — the orthogonal step cost is the ENTERED cell's terrain `move_cost`, and
/// the diagonal step cost is the OCTILE `round(move_cost × √2)`. Asserted by the
/// DERIVATION over ARBITRARY terrain costs (NOT the shipped defaults): build a
/// custom [`MoveCosts`] table, then check each yielded cost against the relation.
#[test]
fn step_cost_orthogonal_is_terrain_diagonal_is_octile() {
    use std::f32::consts::SQRT_2;

    // Arbitrary, NON-default terrain costs — the relation must hold for ANY cost,
    // so we pin the DERIVATION, never a shipped magnitude (C6).
    let open_cost = 7u8;
    let costs = MoveCosts {
        open:  MoveCost::new(open_cost),
        cover: MoveCost::new(11),
        wall:  MoveCost::new(13),
    };
    let grid = OccupancyGrid::new(); // all Open, so every step enters Open terrain
    let origin = key(5, 5, 0);

    let orthogonal = key(6, 5, 0); // due east — an orthogonal step
    let diagonal = key(6, 6, 0); // NE — a diagonal step

    let edges = neighbours_of(origin, &grid, costs);
    let ortho_cost = edges
        .iter()
        .find(|(cell, _)| *cell == (orthogonal.x, orthogonal.y, orthogonal.z))
        .map(|(_, cost)| *cost);
    let diag_cost = edges
        .iter()
        .find(|(cell, _)| *cell == (diagonal.x, diagonal.y, diagonal.z))
        .map(|(_, cost)| *cost);

    // Orthogonal pays the entered cell's move_cost UNCHANGED (the relation, using
    // our arbitrary open_cost — not a shipped number).
    assert_eq!(
        ortho_cost,
        Some(Tu::new(open_cost)),
        "an orthogonal step costs the entered cell's terrain move_cost",
    );
    // Diagonal pays round(move_cost × √2) — the DERIVATION, recomputed here from
    // the same arbitrary open_cost.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "open_cost is a small u8; open_cost * √2 rounds to a value that fits a u8 and is \
                  non-negative, so the cast cannot truncate or sign-flip"
    )]
    let expected_octile = (f32::from(open_cost) * SQRT_2).round() as u8;
    assert_eq!(
        diag_cost,
        Some(Tu::new(expected_octile)),
        "a diagonal step costs round(move_cost × √2) (octile, C2)",
    );
    // And the diagonal is strictly DEARER than the orthogonal for the same terrain
    // (the whole point of octile — flat same-cost diagonals are rejected).
    assert!(
        expected_octile > open_cost,
        "the octile diagonal cost exceeds the orthogonal for the same terrain",
    );
}

/// C5 — neighbour iteration order is DETERMINISTIC, sorted by the `(z, y, x)`
/// cell key (the `auto_select` precedent). Asserted on the OPEN interior cell's
/// full eight neighbours: the emitted sequence equals the `(z, y, x)`-sorted
/// order, and a second call is byte-identical.
#[test]
fn neighbour_order_is_z_y_x_sorted() {
    let grid = OccupancyGrid::new();
    let costs = MoveCosts::default();
    let origin = key(5, 5, 0);

    let cells: Vec<(i32, i32, i32)> = neighbours_of(origin, &grid, costs)
        .into_iter()
        .map(|(cell, _)| cell)
        .collect();

    // The canonical (z, y, x) order: all share z = 0, so it reduces to (y, x).
    let mut sorted = cells.clone();
    sorted.sort_by_key(|&(x, y, z)| (z, y, x));
    assert_eq!(
        cells, sorted,
        "neighbours are emitted in (z, y, x) cell-key order"
    );

    // Replay-stable: a second enumeration is byte-identical.
    let again: Vec<(i32, i32, i32)> = neighbours_of(origin, &grid, costs)
        .into_iter()
        .map(|(cell, _)| cell)
        .collect();
    assert_eq!(cells, again, "enumeration is deterministic across calls");
}

/// An origin whose every planar neighbour is blocked or off-grid yields an EMPTY
/// iterator — never a panic (the graceful-degradation contract).
#[test]
fn fully_walled_origin_yields_no_neighbours() {
    // Wall every one of the eight surrounding cells of (5, 5, 0).
    let walls: Vec<(CellLevel, TerrainKind)> = [
        (4, 4),
        (5, 4),
        (6, 4),
        (4, 5),
        (6, 5),
        (4, 6),
        (5, 6),
        (6, 6),
    ]
    .into_iter()
    .map(|(x, y)| (key(x, y, 0), TerrainKind::Wall))
    .collect();
    let grid = grid_with(&walls);
    let costs = MoveCosts::default();

    assert_eq!(
        neighbours_of(key(5, 5, 0), &grid, costs).len(),
        0,
        "an origin walled on all sides has no pathable neighbours",
    );
}
