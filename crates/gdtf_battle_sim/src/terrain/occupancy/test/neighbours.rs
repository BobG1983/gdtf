//! GTW-12b — `pathable_neighbors` (E7, ADR-0005): same-storey 8-connected planar
//! neighbour enumeration + octile costs + deterministic ordering. Tests are
//! RELATIONS-ONLY (C6): no pinned shipped magnitudes — costs are asserted by
//! their DERIVATION over arbitrary terrain costs, never the
//! `MoveCosts::default()` numbers.

use super::{
    super::{OccupancyGrid, TerrainKind},
    support::*,
};
use crate::{ganger::Tu, metric::CellLevel, terrain::floor::FloorCostGrid, tuning::MoveCost};

/// C2/C4 — on an entirely OPEN grid an interior cell has all EIGHT 8-connected
/// planar neighbours (orthogonal + diagonal), all on the SAME storey, and none
/// blocked.
#[test]
fn open_interior_cell_has_all_eight_planar_neighbours() {
    let grid = OccupancyGrid::new(); // all Open
    let floor_costs = default_floor_costs();
    let origin = key(5, 5, 0);

    let cells: Vec<(i32, i32, i32)> = neighbours_of(origin, &grid, &floor_costs)
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
    let floor_costs = default_floor_costs();
    let origin = key(5, 5, 0);

    assert!(
        !yields_neighbour(origin, blocked, &grid, &floor_costs),
        "a standing wall neighbour must be excluded (C4)",
    );
    assert!(
        yields_neighbour(origin, key(4, 5, 0), &grid, &floor_costs),
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
    let floor_costs = default_floor_costs();
    let origin = key(5, 5, 0);

    // Standing cover blocks → excluded.
    assert!(
        !yields_neighbour(origin, cover, &grid, &floor_costs),
        "a standing cover neighbour must be excluded (C4)",
    );
    // Destroyed cover is walkable → included.
    grid.mark_cover_destroyed(cover);
    assert!(
        yields_neighbour(origin, cover, &grid, &floor_costs),
        "a destroyed-cover neighbour must be walkable (C4)",
    );
}

/// C4 — an OUT-OF-BOUNDS planar offset is EXCLUDED. An origin in the corner
/// `(0, 0, 0)` has its three off-grid offsets (negative x / negative y) dropped,
/// leaving only the three in-bounds neighbours.
#[test]
fn out_of_bounds_neighbours_excluded() {
    let grid = OccupancyGrid::new();
    let floor_costs = default_floor_costs();
    let origin = key(0, 0, 0);

    let cells: Vec<(i32, i32, i32)> = neighbours_of(origin, &grid, &floor_costs)
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
    let floor_costs = default_floor_costs();
    let origin = key(5, 5, 0);
    let diagonal = key(6, 6, 0);

    assert!(
        yields_neighbour(origin, diagonal, &grid, &floor_costs),
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
    let floor_costs = default_floor_costs();
    let origin = key(5, 5, 0);
    let diagonal = key(6, 6, 0);

    // The diagonal cell itself is walkable …
    assert!(
        !*grid.is_blocked(&diagonal),
        "the diagonal cell itself is open"
    );
    // … yet the step is illegal — both shared-edge orthogonals are blocked (C3).
    assert!(
        !yields_neighbour(origin, diagonal, &grid, &floor_costs),
        "no corner-cutting: a diagonal between two blocked cells is excluded (C3)",
    );
}

/// C2 — the orthogonal step cost is the ENTERED cell's floor cost, and the diagonal
/// step cost is the OCTILE `round(floor_cost × √2)`. Asserted by the DERIVATION over
/// ARBITRARY floor costs (NOT the shipped defaults): build a custom [`FloorCostGrid`]
/// with the desired default, then check each yielded cost against the relation.
///
/// GTW-396 Decision B / C1: the cost comes from `FloorCostGrid::cost` for the ENTERED
/// cell, NOT from `move_costs.cost(terrain)` — this test is the correctness pin for
/// that cost source.
#[test]
fn step_cost_orthogonal_is_terrain_diagonal_is_octile() {
    use std::f32::consts::SQRT_2;

    // Arbitrary, NON-default floor cost — the relation must hold for ANY cost,
    // so we pin the DERIVATION, never a shipped magnitude (C6). Build a FloorCostGrid
    // with that cost as the uniform default (all cells same price).
    let open_cost = 7u8;
    let floor_costs = FloorCostGrid::new(MoveCost::new(open_cost), []);
    let grid = OccupancyGrid::new(); // all Open, so every step enters an unoverridden cell
    let origin = key(5, 5, 0);

    let orthogonal = key(6, 5, 0); // due east — an orthogonal step
    let diagonal = key(6, 6, 0); // NE — a diagonal step

    let edges = neighbours_of(origin, &grid, &floor_costs);
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
/// order, and a second call is identical.
#[test]
fn neighbour_order_is_z_y_x_sorted() {
    let grid = OccupancyGrid::new();
    let floor_costs = default_floor_costs();
    let origin = key(5, 5, 0);

    let cells: Vec<(i32, i32, i32)> = neighbours_of(origin, &grid, &floor_costs)
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

    // Replay-stable: a second enumeration is identical.
    let again: Vec<(i32, i32, i32)> = neighbours_of(origin, &grid, &floor_costs)
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
    let floor_costs = default_floor_costs();

    assert_eq!(
        neighbours_of(key(5, 5, 0), &grid, &floor_costs).len(),
        0,
        "an origin walled on all sides has no pathable neighbours",
    );
}
