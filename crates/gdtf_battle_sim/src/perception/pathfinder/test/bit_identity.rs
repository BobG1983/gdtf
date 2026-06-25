//! C6 / §48 bit-identity — the [`Path`](crate::pathfinder::Path) total EQUALS the
//! step-by-step sum of the per-step edge costs along the returned route (the total
//! GTW-355 will charge). Re-derives each step's cost INDEPENDENTLY from the tuning
//! and asserts it matches the search's own total, bit-for-bit.

use super::support::{
    cell, default_floor_costs, grid_with, links_graph, ok_path, stair, step_cost_between,
    summed_step_cost, tuning,
};
use crate::occupancy::TerrainKind;

/// A same-storey route's total equals the summed per-step terrain costs — over a
/// MIXED-terrain grid (open + a destroyed-cover cell on the path), so orthogonal,
/// diagonal-octile, and a dearer-terrain step all contribute.
///
/// GTW-396: `summed_step_cost` now takes a `&FloorCostGrid`; the `default_floor_costs`
/// fixture seeds every cell at the tuning's open cost, which is what the search uses
/// (the grid still controls walkability via `is_blocked`, but the step cost comes from
/// `FloorCostGrid`).
#[test]
fn same_storey_total_equals_summed_steps() {
    // A destroyed-cover cell on the likely path is walkable but priced at the
    // (dearer) cover move-cost — so the route mixes open and cover step costs.
    let mut grid = grid_with(&[(cell(4, 3, 0), TerrainKind::Cover)]);
    grid.mark_cover_destroyed(cell(4, 3, 0));
    let links = super::support::no_links();
    let tuning = tuning();
    let floor_costs = default_floor_costs(&tuning);

    let start = cell(2, 2, 0);
    let goal = cell(6, 4, 0);

    let Some(path) = ok_path(start, goal, &grid, &links, &tuning) else {
        return;
    };

    let summed = summed_step_cost(&path, &floor_costs, &tuning);
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
    let floor_costs = default_floor_costs(&tuning);
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

    let summed = summed_step_cost(&path, &floor_costs, &tuning);
    assert_eq!(
        u32::from(*path.total()),
        summed,
        "the cross-storey Path total equals the summed steps including the link hop (§48)",
    );
    // And it is non-trivial: a multi-step route with a real accumulated cost.
    assert!(*path.total() > 0, "a real route has a positive total");
    assert!(path.cells().len() > 2, "a real multi-step route");
}

/// GTW-355 — the per-step `Path::steps()` the stepped walk charges ARE the §48
/// bit-identity source: each carried step equals the independently re-derived edge
/// cost between its cell pair (terrain step or link hop), the steps are aligned to
/// `cells[1..]` (one per edge), and their sum is the route total bit-for-bit. A
/// wrong per-step cost source (e.g. re-running the octile math off the wrong cell)
/// would break this — it is pin-discriminating.
#[test]
fn carried_per_step_costs_match_each_edge_and_sum_to_total() {
    // The cross-storey mixed route: orthogonal terrain steps PLUS a flat link hop, so
    // both edge kinds appear in `steps()`.
    let grid = grid_with(&[]);
    let tuning = tuning();
    let floor_costs = default_floor_costs(&tuning);
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

    // One carried step per traversed edge — aligned to `cells[1..]`.
    assert_eq!(
        path.steps().len(),
        path.cells().len().saturating_sub(1),
        "steps() carries exactly one entry per edge (aligned to cells[1..])",
    );

    // Each carried step equals the INDEPENDENTLY re-derived edge cost between its pair.
    for (edge, pair) in path.steps().iter().zip(path.cells().windows(2)) {
        let [from, to] = pair else { continue };
        assert_eq!(
            *edge,
            step_cost_between(*from, *to, &floor_costs, &tuning),
            "each carried per-step cost equals the edge cost between its consecutive cells",
        );
    }

    // The carried steps sum to the route total bit-for-bit (the walk-charge identity).
    let carried_sum: u32 = path.steps().iter().map(|step| u32::from(**step)).sum();
    assert_eq!(
        carried_sum,
        u32::from(*path.total()),
        "the carried per-step costs sum to the Path total bit-for-bit (§48)",
    );
    // And it agrees with the independent terrain/link re-derivation.
    assert_eq!(
        carried_sum,
        summed_step_cost(&path, &floor_costs, &tuning),
        "the carried steps agree with the independent per-edge re-derivation",
    );
}
