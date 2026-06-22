//! C6 / §48 bit-identity — the [`Path`](crate::pathfinder::Path) total EQUALS the
//! step-by-step sum of the per-step edge costs along the returned route (the total
//! GTW-355 will charge). Re-derives each step's cost INDEPENDENTLY from the tuning
//! and asserts it matches the search's own total, bit-for-bit.

use super::support::{cell, grid_with, links_graph, ok_path, stair, summed_step_cost, tuning};
use crate::occupancy::TerrainKind;

/// A same-storey route's total equals the summed per-step terrain costs — over a
/// MIXED-terrain grid (open + a destroyed-cover cell on the path), so orthogonal,
/// diagonal-octile, and a dearer-terrain step all contribute.
#[test]
fn same_storey_total_equals_summed_steps() {
    // A destroyed-cover cell on the likely path is walkable but priced at the
    // (dearer) cover move-cost — so the route mixes open and cover step costs.
    let mut grid = grid_with(&[(cell(4, 3, 0), TerrainKind::Cover)]);
    grid.mark_cover_destroyed(cell(4, 3, 0));
    let links = super::support::no_links();
    let tuning = tuning();

    let start = cell(2, 2, 0);
    let goal = cell(6, 4, 0);

    let Some(path) = ok_path(start, goal, &grid, &links, &tuning) else {
        return;
    };

    let summed = summed_step_cost(&path, &grid, &tuning);
    assert_eq!(
        u32::from(*path.total()),
        summed,
        "the Path total equals the independently summed per-step edge costs (§48)",
    );
}

/// A cross-storey route's total equals the summed steps INCLUDING the flat link hop
/// — the link cost accumulates into the total exactly as a terrain step would.
#[test]
fn cross_storey_total_equals_summed_steps_including_link() {
    let grid = grid_with(&[]);
    let tuning = tuning();
    let foot = cell(5, 5, 0);
    let head = cell(5, 5, 1);
    let Some(links) = links_graph(&[stair(foot, head)]) else {
        return;
    };

    let start = cell(2, 5, 0);
    let goal = cell(8, 5, 1);

    let Some(path) = ok_path(start, goal, &grid, &links, &tuning) else {
        return;
    };

    let summed = summed_step_cost(&path, &grid, &tuning);
    assert_eq!(
        u32::from(*path.total()),
        summed,
        "the cross-storey Path total equals the summed steps including the link hop (§48)",
    );
    // And it is non-trivial: a multi-step route with a real accumulated cost.
    assert!(*path.total() > 0, "a real route has a positive total");
    assert!(path.cells().len() > 2, "a real multi-step route");
}
