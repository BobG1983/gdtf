//! C6 — a same-storey point-to-point route: reaches the goal, runs `start..=goal`
//! in order, and detours around a blocking wall (never through it).

use super::support::{cell, grid_with, no_links, ok_path, open_step, tuning};
use crate::occupancy::TerrainKind;

/// A straight-line same-storey route on an open grid runs `start..=goal` in order,
/// every consecutive pair is 8-adjacent on the same storey, and the endpoints match.
#[test]
fn open_grid_route_runs_start_to_goal_in_order() {
    let grid = grid_with(&[]); // all Open
    let links = no_links();
    let tuning = tuning();

    let start = cell(2, 2, 0);
    let goal = cell(6, 2, 0);

    let Some(path) = ok_path(start, goal, &grid, &links, &tuning) else {
        return;
    };

    // The route is start..=goal in step order.
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

    // Every consecutive pair is an 8-connected SAME-storey step (no teleporting,
    // no storey change without a link).
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

/// A wall across the direct line forces a DETOUR: the route still reaches the goal,
/// and never steps onto the blocked wall cell.
#[test]
fn route_detours_around_a_wall() {
    // A vertical wall segment at x = 4 spanning y = 1..=3 blocks the straight path
    // from (2, 2) to (6, 2). The route must go around it.
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
    // The route never steps onto a wall cell.
    for &c in path.cells() {
        assert!(
            !grid.is_blocked(&c),
            "the route never enters a blocked cell ({c:?})",
        );
    }
    // The route never passes through the blocked direct corridor cell (4, 2, 0) —
    // proof it genuinely went AROUND the wall, not through it.
    assert!(
        !path.cells().contains(&cell(4, 2, 0)),
        "the route detours around the wall, never through (4, 2, 0)",
    );
    // The detour is strictly DEARER than the (now-blocked) 4-step straight shot:
    // going around forces octile diagonals / extra steps, so the total exceeds
    // `4 × open` (the cost the straight line would have been, were it clear).
    let straight_line_cost = u32::from(*open_step(&tuning)) * 4;
    assert!(
        u32::from(*path.total()) > straight_line_cost,
        "the detour costs more than the (blocked) straight route would have",
    );
}

/// The degenerate `start == goal` route is the single-cell path `[start]` at total
/// zero — never an empty path, never a panic.
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
