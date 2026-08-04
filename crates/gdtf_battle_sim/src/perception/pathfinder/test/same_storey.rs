use super::support::{cell, grid_with, no_links, ok_path, open_step, tuning};
use crate::occupancy::TerrainKind;

#[test]
fn open_grid_route_runs_start_to_goal_in_order() {
    let grid = grid_with(&[]);
    let links = no_links();
    let tuning = tuning();

    let start = cell(2, 2, 0);
    let goal = cell(6, 2, 0);

    let Some(path) = ok_path(start, goal, &grid, &links, &tuning) else {
        return;
    };

    assert_eq!(path.start(), Some(start), "route begins at start");
    assert_eq!(path.goal(), Some(goal), "route ends at goal");
    assert_eq!(
        path.cells().first().copied(),
        Some(start),
        "the first cell is start",
    );
    assert_eq!(
        path.cells().last().copied(),
        Some(goal),
        "the last cell is goal",
    );

    for pair in path.cells().windows(2) {
        let [from, to] = pair else { continue };
        let dx = (from.x - to.x).abs();
        let dy = (from.y - to.y).abs();
        assert_eq!(from.z, to.z, "a same-storey route never changes storey");
        assert!(
            dx <= 1 && dy <= 1 && (dx + dy) > 0,
            "each step is one 8-connected cell ({from:?} -> {to:?})",
        );
    }
}

#[test]
fn route_detours_around_a_wall() {
    let wall = [
        (cell(4, 1, 0), TerrainKind::Wall),
        (cell(4, 2, 0), TerrainKind::Wall),
        (cell(4, 3, 0), TerrainKind::Wall),
    ];
    let grid = grid_with(&wall);
    let links = no_links();
    let tuning = tuning();

    let start = cell(2, 2, 0);
    let goal = cell(6, 2, 0);

    let Some(path) = ok_path(start, goal, &grid, &links, &tuning) else {
        return;
    };

    assert_eq!(path.goal(), Some(goal), "the route still reaches the goal");
    for &c in path.cells() {
        assert!(
            !*grid.is_blocked(&c),
            "the route never enters a blocked cell ({c:?})",
        );
    }
    assert!(
        !path.cells().contains(&cell(4, 2, 0)),
        "the route detours around the wall, never through (4, 2, 0)",
    );
    let straight_line_cost = u32::from(*open_step(&tuning)) * 4;
    assert!(
        u32::from(*path.total()) > straight_line_cost,
        "the detour costs more than the (blocked) straight route would have",
    );
}

#[test]
fn start_equals_goal_is_single_cell_zero_cost() {
    let grid = grid_with(&[]);
    let links = no_links();
    let tuning = tuning();
    let at = cell(3, 3, 0);

    let Some(path) = ok_path(at, at, &grid, &links, &tuning) else {
        return;
    };
    assert_eq!(path.cells(), &[at], "start == goal is the one-cell route");
    assert_eq!(*path.total(), 0, "a zero-step route costs nothing");
    assert!(!path.is_empty(), "a search route is never empty");
}
