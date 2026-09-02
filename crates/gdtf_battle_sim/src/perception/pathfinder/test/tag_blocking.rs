use super::support::{
    all_other, cell, default_floor_costs, full_vision, grid_with, grid_with_path_blocking,
    no_links, tuning,
};
use crate::{
    injuries::MovementCostFactor,
    occupancy::{OccupancyGrid, TerrainKind},
    pathfinder::{MoveGrids, PathBlocked, PlanningView, find_path},
};

fn corridor_reaches_goal(grid: &OccupancyGrid) -> bool {
    let links = no_links();
    let tuning = tuning();
    let floor_costs = default_floor_costs(&tuning);
    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    find_path(
        cell(0, 0, 0),
        cell(0, 2, 0),
        MoveGrids {
            occupancy:   grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        MovementCostFactor::IDENTITY,
        &planning,
    )
    .is_ok()
}

fn walled_corridor() -> OccupancyGrid {
    grid_with(&[
        (cell(1, 0, 0), TerrainKind::Wall),
        (cell(1, 1, 0), TerrainKind::Wall),
        (cell(1, 2, 0), TerrainKind::Wall),
    ])
}

#[test]
fn explicitly_path_blocked_cell_blocks_the_route() {
    let mut grid = walled_corridor();
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
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        MovementCostFactor::IDENTITY,
        &planning,
    );
    assert_eq!(
        result,
        Err(PathBlocked),
        "C6(a): a tag-path-blocked chokepoint makes the only route PathBlocked",
    );
}

#[test]
fn same_cell_without_marker_does_not_block() {
    let grid = walled_corridor();
    assert!(
        corridor_reaches_goal(&grid),
        "C6(b): with no path-blocking marker on the chokepoint, the route succeeds",
    );
}

#[test]
fn path_blocking_surface_decides_in_isolation() {
    let blocked =
        grid_with_path_blocking(&[cell(1, 0, 0), cell(1, 1, 0), cell(1, 2, 0), cell(0, 1, 0)]);
    assert!(
        !corridor_reaches_goal(&blocked),
        "the tag-derived surface alone blocks the chokepoint → PathBlocked",
    );

    let open = grid_with_path_blocking(&[cell(1, 0, 0), cell(1, 1, 0), cell(1, 2, 0)]);
    assert!(
        corridor_reaches_goal(&open),
        "removing the chokepoint marker re-opens the route (surface decides, not kind)",
    );
}

#[test]
fn kind_default_wall_still_blocks() {
    let grid = grid_with(&[
        (cell(1, 0, 0), TerrainKind::Wall),
        (cell(1, 1, 0), TerrainKind::Wall),
        (cell(1, 2, 0), TerrainKind::Wall),
        (cell(0, 1, 0), TerrainKind::Wall),
    ]);
    assert!(
        !corridor_reaches_goal(&grid),
        "C6(c): a kind-default Wall on the chokepoint still makes the route PathBlocked",
    );
}

#[test]
fn a_cell_whose_path_blocking_was_cleared_does_not_block() {
    let mut grid = walled_corridor();
    grid.set_path_blocking(cell(0, 1, 0));
    grid.clear_path_blocking(cell(0, 1, 0));
    assert!(
        corridor_reaches_goal(&grid),
        "a destroyed piece loses BlocksPathfinding, project_path_blocking clears the cell it \
         tracked, and the route re-opens",
    );
}
