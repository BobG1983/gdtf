use super::{
    super::{OccupancyGrid, TerrainKind},
    support::*,
};
use crate::{ganger::Tu, metric::CellLevel, terrain::floor::FloorCostGrid, tuning::MoveCost};

#[test]
fn open_interior_cell_has_all_eight_planar_neighbours() {
    let grid = OccupancyGrid::new();
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
    assert!(
        !cells.contains(&(5, 5, 0)),
        "a cell is not its own neighbour"
    );
    assert!(
        cells.iter().all(|(_, _, z)| *z == origin.z),
        "every planar neighbour shares the origin's storey",
    );
}

#[test]
fn blocked_neighbour_excluded_open_included() {
    let blocked = key(6, 5, 0);
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

#[test]
fn destroyed_cover_neighbour_is_walkable() {
    let cover = key(6, 5, 0);
    let mut grid = grid_with(&[(cover, TerrainKind::Cover)]);
    let floor_costs = default_floor_costs();
    let origin = key(5, 5, 0);

    assert!(
        !yields_neighbour(origin, cover, &grid, &floor_costs),
        "a standing cover neighbour must be excluded (C4)",
    );
    grid.mark_cover_destroyed(cover);
    assert!(
        yields_neighbour(origin, cover, &grid, &floor_costs),
        "a destroyed-cover neighbour must be walkable (C4)",
    );
}

#[test]
fn out_of_bounds_neighbours_excluded() {
    let grid = OccupancyGrid::new();
    let floor_costs = default_floor_costs();
    let origin = key(0, 0, 0);

    let cells: Vec<(i32, i32, i32)> = neighbours_of(origin, &grid, &floor_costs)
        .into_iter()
        .map(|(cell, _)| cell)
        .collect();

    assert_eq!(
        cells.len(),
        3,
        "a corner cell has only 3 in-bounds neighbours"
    );
    assert!(cells.contains(&(1, 0, 0)));
    assert!(cells.contains(&(0, 1, 0)));
    assert!(cells.contains(&(1, 1, 0)));
    assert!(
        cells.iter().all(|(x, y, _)| *x >= 0 && *y >= 0),
        "no out-of-bounds (negative) neighbour is yielded",
    );
}

#[test]
fn diagonal_present_when_one_shared_edge_walkable() {
    let grid = grid_with(&[(key(6, 5, 0), TerrainKind::Wall)]);
    let floor_costs = default_floor_costs();
    let origin = key(5, 5, 0);
    let diagonal = key(6, 6, 0);

    assert!(
        yields_neighbour(origin, diagonal, &grid, &floor_costs),
        "a diagonal must survive when one shared-edge orthogonal is walkable (C3)",
    );
}

#[test]
fn diagonal_absent_when_both_shared_edges_blocked() {
    let grid = grid_with(&[
        (key(6, 5, 0), TerrainKind::Wall),
        (key(5, 6, 0), TerrainKind::Wall),
    ]);
    let floor_costs = default_floor_costs();
    let origin = key(5, 5, 0);
    let diagonal = key(6, 6, 0);

    assert!(
        !*grid.is_blocked(&diagonal),
        "the diagonal cell itself is open"
    );
    assert!(
        !yields_neighbour(origin, diagonal, &grid, &floor_costs),
        "no corner-cutting: a diagonal between two blocked cells is excluded (C3)",
    );
}

#[test]
fn step_cost_orthogonal_is_terrain_diagonal_is_octile() {
    use std::f32::consts::SQRT_2;

    let open_cost = 7u8;
    let floor_costs = FloorCostGrid::new(MoveCost::new(open_cost), []);
    let grid = OccupancyGrid::new();
    let origin = key(5, 5, 0);

    let orthogonal = key(6, 5, 0);
    let diagonal = key(6, 6, 0);

    let edges = neighbours_of(origin, &grid, &floor_costs);
    let ortho_cost = edges
        .iter()
        .find(|(cell, _)| *cell == (orthogonal.x, orthogonal.y, orthogonal.z))
        .map(|(_, cost)| *cost);
    let diag_cost = edges
        .iter()
        .find(|(cell, _)| *cell == (diagonal.x, diagonal.y, diagonal.z))
        .map(|(_, cost)| *cost);

    assert_eq!(
        ortho_cost,
        Some(Tu::new(open_cost)),
        "an orthogonal step costs the entered cell's terrain move_cost",
    );
    let expected_octile = (f32::from(open_cost) * SQRT_2).round() as u8;
    assert_eq!(
        diag_cost,
        Some(Tu::new(expected_octile)),
        "a diagonal step costs round(move_cost × √2) (octile, C2)",
    );
    assert!(
        expected_octile > open_cost,
        "the octile diagonal cost exceeds the orthogonal for the same terrain",
    );
}

#[test]
fn neighbour_order_is_z_y_x_sorted() {
    let grid = OccupancyGrid::new();
    let floor_costs = default_floor_costs();
    let origin = key(5, 5, 0);

    let cells: Vec<(i32, i32, i32)> = neighbours_of(origin, &grid, &floor_costs)
        .into_iter()
        .map(|(cell, _)| cell)
        .collect();

    let mut sorted = cells.clone();
    sorted.sort_by_key(|&(x, y, z)| (z, y, x));
    assert_eq!(
        cells, sorted,
        "neighbours are emitted in (z, y, x) cell-key order"
    );

    let again: Vec<(i32, i32, i32)> = neighbours_of(origin, &grid, &floor_costs)
        .into_iter()
        .map(|(cell, _)| cell)
        .collect();
    assert_eq!(cells, again, "enumeration is deterministic across calls");
}

#[test]
fn fully_walled_origin_yields_no_neighbours() {
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
