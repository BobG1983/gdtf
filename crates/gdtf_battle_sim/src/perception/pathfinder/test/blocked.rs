use super::support::{
    all_other, cell, default_floor_costs, full_vision, grid_with, no_links, tuning,
};
use crate::{
    occupancy::TerrainKind,
    pathfinder::{PathBlocked, PlanningView, find_path},
};

#[test]
fn goal_walled_in_is_path_blocked() {
    let walls: Vec<_> = [
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
    .map(|(x, y)| (cell(x, y, 0), TerrainKind::Wall))
    .collect();
    let grid = grid_with(&walls);
    let links = no_links();
    let tuning = tuning();

    let start = cell(0, 0, 0);
    let goal = cell(5, 5, 0);

    let floor_costs = default_floor_costs(&tuning);
    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    let result = find_path(
        start,
        goal,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    assert_eq!(
        result,
        Err(PathBlocked),
        "a walled-in goal is a typed PathBlocked, not a panic",
    );
}

#[test]
fn other_storey_without_link_is_path_blocked() {
    let grid = grid_with(&[]);
    let links = no_links();
    let tuning = tuning();

    let start = cell(2, 2, 0);
    let goal = cell(2, 2, 3);

    let floor_costs = default_floor_costs(&tuning);
    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    let result = find_path(
        start,
        goal,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    assert_eq!(
        result,
        Err(PathBlocked),
        "another storey with no link is PathBlocked",
    );
}

#[test]
fn out_of_grid_goal_is_path_blocked() {
    let grid = grid_with(&[]);
    let links = no_links();
    let tuning = tuning();

    let start = cell(1, 1, 0);
    let goal = cell(-5, -5, 0);

    let floor_costs = default_floor_costs(&tuning);
    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    let result = find_path(
        start,
        goal,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    assert_eq!(result, Err(PathBlocked), "an off-grid goal is PathBlocked");
}
