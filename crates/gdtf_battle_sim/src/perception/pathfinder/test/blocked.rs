//! C6 — an unreachable goal yields the typed [`PathBlocked`](crate::pathfinder::PathBlocked)
//! `Err`, NEVER a panic and NEVER an empty `Path`.

use super::support::{all_other, cell, full_vision, grid_with, no_links, tuning};
use crate::{
    occupancy::TerrainKind,
    pathfinder::{PathBlocked, PlanningView, find_path},
};

/// A goal walled off on all eight sides is UNREACHABLE — `find_path` returns
/// `Err(PathBlocked)`, not a panic and not an empty route.
#[test]
fn goal_walled_in_is_path_blocked() {
    // Wall every one of the eight cells surrounding the goal (5, 5, 0).
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
    let goal = cell(5, 5, 0); // open, but boxed in by walls

    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    let result = find_path(start, goal, &grid, &links, &tuning, &planning);
    assert_eq!(
        result,
        Err(PathBlocked),
        "a walled-in goal is a typed PathBlocked, not a panic",
    );
}

/// A goal on a DIFFERENT storey with no vertical link is unreachable — the typed
/// `PathBlocked`. (Storeys are stitched ONLY by the link graph.)
#[test]
fn other_storey_without_link_is_path_blocked() {
    let grid = grid_with(&[]);
    let links = no_links();
    let tuning = tuning();

    let start = cell(2, 2, 0);
    let goal = cell(2, 2, 3); // three storeys up, no link

    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    let result = find_path(start, goal, &grid, &links, &tuning, &planning);
    assert_eq!(
        result,
        Err(PathBlocked),
        "another storey with no link is PathBlocked",
    );
}

/// An out-of-grid goal is unreachable (the grid's bounds gate drops every edge that
/// would leave the grid) — a typed `PathBlocked`, never a panic.
#[test]
fn out_of_grid_goal_is_path_blocked() {
    let grid = grid_with(&[]);
    let links = no_links();
    let tuning = tuning();

    let start = cell(1, 1, 0);
    let goal = cell(-5, -5, 0); // off the grid

    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    let result = find_path(start, goal, &grid, &links, &tuning, &planning);
    assert_eq!(result, Err(PathBlocked), "an off-grid goal is PathBlocked");
}
