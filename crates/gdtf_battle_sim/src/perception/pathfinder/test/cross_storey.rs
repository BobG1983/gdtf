//! C6 — a cross-storey route THROUGH a vertical link: the route changes storey
//! across the link hop, and the `link_tu` ACCUMULATES into the total (the §48
//! cost-accumulation check).

use super::support::{
    all_other, cell, default_floor_costs, full_vision, grid_with, link_step, links_graph, ok_path,
    stair, summed_step_cost, tuning,
};
use crate::pathfinder::PlanningView;

/// A route from a cell on storey 0 to a cell on storey 1 traverses the authored
/// stair link, changes storey across exactly that hop, and the total accumulates
/// the `link_tu` for the crossing PLUS the terrain steps either side.
#[test]
fn route_traverses_vertical_link_and_accumulates_link_tu() {
    let grid = grid_with(&[]); // all Open on both storeys
    let tuning = tuning();

    // The stair connects (5, 5, 0) <-> (5, 5, 1); the route runs from (3, 5, 0) up
    // to (7, 5, 1), so it must reach the link foot, climb, then walk on storey 1.
    let foot = cell(5, 5, 0);
    let head = cell(5, 5, 1);
    let Some(links) = links_graph(&[stair(foot, head)]) else {
        return;
    };

    let start = cell(3, 5, 0);
    let goal = cell(7, 5, 1);

    let Some(path) = ok_path(start, goal, &grid, &links, &tuning) else {
        return;
    };

    assert_eq!(path.start(), Some(start));
    assert_eq!(
        path.goal(),
        Some(goal),
        "the route reaches the upper storey"
    );

    // The route changes storey EXACTLY once, and only across the authored link
    // endpoints (foot -> head).
    let mut storey_changes = 0;
    for pair in path.cells().windows(2) {
        let [from, to] = pair else { continue };
        if from.z != to.z {
            storey_changes += 1;
            // The storey change is the authored link hop (either direction).
            let crosses_link = (*from == foot && *to == head) || (*from == head && *to == foot);
            assert!(
                crosses_link,
                "a storey change only happens across the authored link ({from:?} -> {to:?})",
            );
        }
    }
    assert_eq!(
        storey_changes, 1,
        "the route crosses exactly one storey boundary",
    );

    // The total ACCUMULATES the link hop: it must be at least the flat link cost
    // (the crossing) plus a positive terrain walk on both storeys.
    assert!(
        *path.total() > *link_step(&tuning),
        "the total accumulates the link_tu hop PLUS the terrain steps",
    );

    // And the §48 cost-accumulation: the route total equals the summed per-step
    // edge costs (link hop + terrain steps), re-derived independently.
    // GTW-396: summed_step_cost now reads from FloorCostGrid instead of the grid.
    let floor_costs = default_floor_costs(&tuning);
    let summed = summed_step_cost(&path, &floor_costs, &tuning);
    assert_eq!(
        u32::from(*path.total()),
        summed,
        "the cross-storey total equals the summed per-step costs (link + terrain)",
    );
}

/// Removing the only link makes the upper-storey goal UNREACHABLE — proving the
/// route above genuinely depends on the vertical link (storeys are stitched ONLY by
/// the link graph, never by adjacency).
#[test]
fn upper_storey_unreachable_without_a_link() {
    let grid = grid_with(&[]);
    let tuning = tuning();
    let links = super::support::no_links(); // NO vertical links

    let start = cell(3, 5, 0);
    let goal = cell(7, 5, 1);

    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    let floor_costs = default_floor_costs(&tuning);
    let result = crate::pathfinder::find_path(
        start,
        goal,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    assert!(
        result.is_err(),
        "without a vertical link, another storey is unreachable",
    );
}

/// The single link hop itself costs exactly the flat `link_tu` (relations-only):
/// routing from the link foot to its head is a one-hop route whose total is the
/// link cost, NOT a terrain step.
#[test]
fn single_link_hop_costs_exactly_link_tu() {
    let grid = grid_with(&[]);
    let tuning = tuning();
    let foot = cell(4, 4, 0);
    let head = cell(4, 4, 1);
    let Some(links) = links_graph(&[stair(foot, head)]) else {
        return;
    };

    let Some(path) = ok_path(foot, head, &grid, &links, &tuning) else {
        return;
    };
    assert_eq!(
        path.cells(),
        &[foot, head],
        "the route is the two link endpoints",
    );
    assert_eq!(
        *path.total(),
        *link_step(&tuning),
        "a single link hop costs exactly the flat link_tu",
    );
}
