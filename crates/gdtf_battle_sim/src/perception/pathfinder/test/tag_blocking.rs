//! GTW-501 (C6 a/b/c) — the pathfinder reads the **tag-derived path-blocking surface**,
//! not [`TerrainKind`](crate::occupancy::TerrainKind).
//!
//! The discriminating pair: ONE cell on the sole route either carries a path-blocking
//! marker (→ [`PathBlocked`](crate::pathfinder::PathBlocked)) or does NOT (→ a route). Plus
//! the zero-regression check that a kind-default `Wall` still blocks. These are PURE
//! pathfinder tests over a hand-built [`OccupancyGrid`](crate::occupancy::OccupancyGrid); the
//! change-detection drive of the REAL projection/spawn systems is the headless integration
//! test (GTW-501 C6d).

use super::support::{
    all_other, cell, default_floor_costs, full_vision, grid_with, grid_with_path_blocking,
    no_links, tuning,
};
use crate::{
    injuries::MovementCostFactor,
    occupancy::{OccupancyGrid, TerrainKind},
    pathfinder::{PathBlocked, PlanningView, find_path},
};

/// Route the corridor start→goal over `grid`, returning whether `find_path` reached the
/// goal — the structural fact the GTW-501 discriminator asserts (route found vs
/// `PathBlocked`), never an exact cost (the brittle-test rule).
fn corridor_reaches_goal(grid: &OccupancyGrid) -> bool {
    // A 1-wide corridor up the x = 0 column: x = 1 is walled, x = -1 is off-grid, so the
    // ONLY route from (0,0,0) to (0,2,0) passes through the chokepoint (0,1,0).
    let links = no_links();
    let tuning = tuning();
    let floor_costs = default_floor_costs(&tuning);
    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    find_path(
        cell(0, 0, 0),
        cell(0, 2, 0),
        grid,
        &links,
        &tuning,
        &floor_costs,
        MovementCostFactor::IDENTITY,
        &planning,
    )
    .is_ok()
}

/// A grid that walls off the x = 1 column beside the corridor, so the sole route from
/// (0,0,0) to (0,2,0) is the straight x = 0 column through the chokepoint (0,1,0). The
/// chokepoint itself is left OPEN here; the tests then add/omit the path-blocking marker.
fn walled_corridor() -> OccupancyGrid {
    grid_with(&[
        (cell(1, 0, 0), TerrainKind::Wall),
        (cell(1, 1, 0), TerrainKind::Wall),
        (cell(1, 2, 0), TerrainKind::Wall),
    ])
}

/// C6(a) — a cell EXPLICITLY tagged path-blocking (here: a `Slab`-kind cell, OPEN to the
/// kind-based occupancy, that carries a `BlocksPathfinding` marker projected into the
/// surface) blocks an otherwise-valid path: the chokepoint is impassable, so the goal is
/// `PathBlocked`.
#[test]
fn explicitly_path_blocked_cell_blocks_the_route() {
    let mut grid = walled_corridor();
    // The chokepoint is path-blocking via the TAG-derived surface, NOT via TerrainKind:
    // its kind stays Open (a Slab is Open to occupancy), yet the path is barred.
    grid.set_path_blocking(cell(0, 1, 0));
    assert_eq!(
        grid.terrain(&cell(0, 1, 0)),
        TerrainKind::Open,
        "C6(a): the chokepoint's KIND is Open — only the tag-derived surface blocks it",
    );

    let links = no_links();
    let tuning = tuning();
    let floor_costs = default_floor_costs(&tuning);
    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    let result = find_path(
        cell(0, 0, 0),
        cell(0, 2, 0),
        &grid,
        &links,
        &tuning,
        &floor_costs,
        MovementCostFactor::IDENTITY,
        &planning,
    );
    assert_eq!(
        result,
        Err(PathBlocked),
        "C6(a): a tag-path-blocked chokepoint makes the only route PathBlocked",
    );
}

/// C6(b) — the SAME cell WITHOUT the path-blocking marker does NOT block: the corridor is
/// walkable, so `find_path` reaches the goal. The discriminating partner of C6(a) — the
/// ONLY difference is the marker.
#[test]
fn same_cell_without_marker_does_not_block() {
    // Identical walled corridor, but NO path-blocking marker on the chokepoint.
    let grid = walled_corridor();
    assert!(
        corridor_reaches_goal(&grid),
        "C6(b): with no path-blocking marker on the chokepoint, the route succeeds",
    );
}

/// C6(b·isolation) — the surface read is purely the tag-derived set: a grid with ONLY the
/// chokepoint marked path-blocking (and the side walls likewise marked, so the corridor is
/// the sole route) is `PathBlocked`, while the same grid without the chokepoint marker
/// routes. Exercises [`grid_with_path_blocking`] (no `TerrainKind` involved at all).
#[test]
fn path_blocking_surface_decides_in_isolation() {
    // The side walls AND the chokepoint are path-blocking via the surface only.
    let blocked =
        grid_with_path_blocking(&[cell(1, 0, 0), cell(1, 1, 0), cell(1, 2, 0), cell(0, 1, 0)]);
    assert!(
        !corridor_reaches_goal(&blocked),
        "the tag-derived surface alone blocks the chokepoint → PathBlocked",
    );

    // Same surface MINUS the chokepoint marker → the corridor is open again.
    let open = grid_with_path_blocking(&[cell(1, 0, 0), cell(1, 1, 0), cell(1, 2, 0)]);
    assert!(
        corridor_reaches_goal(&open),
        "removing the chokepoint marker re-opens the route (surface decides, not kind)",
    );
}

/// C6(c) — a `Wall` (kind-default path-blocking) STILL blocks: a wall on the chokepoint
/// makes the goal `PathBlocked`, exactly as before the GTW-501 split (the zero-regression
/// guarantee, C5). `grid_with` mirrors a `Wall` placement into the path-blocking surface, so
/// existing wall geometry routes identically.
#[test]
fn kind_default_wall_still_blocks() {
    let grid = grid_with(&[
        (cell(1, 0, 0), TerrainKind::Wall),
        (cell(1, 1, 0), TerrainKind::Wall),
        (cell(1, 2, 0), TerrainKind::Wall),
        // The chokepoint is a Wall — kind-default path-blocking.
        (cell(0, 1, 0), TerrainKind::Wall),
    ]);
    assert!(
        !corridor_reaches_goal(&grid),
        "C6(c): a kind-default Wall on the chokepoint still makes the route PathBlocked",
    );
}

/// C6(c·destroyed) — a destroyed cover cell does NOT block the path even with a marker: the
/// surface mirrors the destroyed-cover exclusion (C5), so a smashed wall/prop re-opens the
/// route exactly as it does for vision. Drives [`is_path_blocked`] through the destroyed-cover
/// branch.
#[test]
fn destroyed_marked_cell_does_not_block() {
    let mut grid = walled_corridor();
    grid.set_path_blocking(cell(0, 1, 0));
    // Smash it: a destroyed cell is excluded from is_path_blocked even though the marker is
    // still set (the C5 mirror of is_blocked's destroyed-cover exclusion).
    grid.mark_cover_destroyed(cell(0, 1, 0));
    assert!(
        corridor_reaches_goal(&grid),
        "C6(c·destroyed): a destroyed path-blocking cell re-opens the route",
    );
}
