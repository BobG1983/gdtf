//! C6 — DETERMINISM replay: the same inputs produce a byte-identical
//! [`Path`](crate::pathfinder::Path) AND a byte-identical
//! [`reachable_within`](crate::pathfinder::reachable_within) set, asserted across
//! repeated calls. No RNG; the `(cost, (z, y, x) cell_key)` frontier tie-break + the
//! pre-sorted GTW-350/351 edge enumeration pin the result.

use super::support::{
    all_other, cell, default_floor_costs, full_vision, grid_with, links_graph, reachable_triples,
    stair, tuning,
};
use crate::{
    ganger::Tu,
    occupancy::TerrainKind,
    pathfinder::{PlanningView, find_path, reachable_within},
};

/// `find_path` over a non-trivial multi-storey grid (a wall to detour, a link to
/// climb) returns the SAME route on every call — byte-identical `Path` (cells AND
/// total), proving the search is replay-stable.
#[test]
fn find_path_is_byte_identical_across_replays() {
    let wall = [
        (cell(4, 1, 0), TerrainKind::Wall),
        (cell(4, 2, 0), TerrainKind::Wall),
        (cell(4, 3, 0), TerrainKind::Wall),
    ];
    let grid = grid_with(&wall);
    let tuning = tuning();
    let Some(links) = links_graph(&[stair(cell(7, 2, 0), cell(7, 2, 1))]) else {
        return;
    };

    let start = cell(1, 2, 0);
    let goal = cell(9, 2, 1);

    let floor_costs = default_floor_costs(&tuning);
    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    let first = find_path(start, goal, &grid, &links, &tuning, &floor_costs, &planning);
    let second = find_path(start, goal, &grid, &links, &tuning, &floor_costs, &planning);
    let third = find_path(start, goal, &grid, &links, &tuning, &floor_costs, &planning);

    assert!(first.is_ok(), "the fixture has a route: {first:?}");
    // Byte-identical across all three replays (the whole Path: cells + total).
    assert_eq!(first, second, "replay 2 differs from replay 1");
    assert_eq!(second, third, "replay 3 differs from replay 2");
}

/// `reachable_within` over the same multi-storey grid returns the SAME reachable set
/// (cells AND costs, in the SAME sorted order) on every call — byte-identical, so the
/// GTW-357 overlay it feeds is replay-stable.
#[test]
fn reachable_within_is_byte_identical_across_replays() {
    let wall = [
        (cell(4, 4, 0), TerrainKind::Wall),
        (cell(4, 5, 0), TerrainKind::Wall),
    ];
    let grid = grid_with(&wall);
    let tuning = tuning();
    let Some(links) = links_graph(&[stair(cell(6, 4, 0), cell(6, 4, 1))]) else {
        return;
    };

    let start = cell(5, 5, 0);
    let budget = Tu::new(20);

    let floor_costs = default_floor_costs(&tuning);
    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    let first = reachable_within(
        start,
        budget,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        &planning,
    );
    let second = reachable_within(
        start,
        budget,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        &planning,
    );

    // Byte-identical: same cells, same costs, same ORDER (the (z, y, x) sort).
    assert_eq!(
        first, second,
        "the reachable set must be byte-identical across replays (same cells, costs, order)",
    );
    assert!(!first.is_empty(), "the start reaches at least itself");

    // The set is sorted by the (z, y, x) cell key — the data-only total order.
    let triples = reachable_triples(start, budget, &grid, &links, &tuning);
    let mut sorted = triples.clone();
    sorted.sort_by_key(|((x, y, z), _)| (*z, *y, *x));
    assert_eq!(
        triples, sorted,
        "the reachable set is emitted in (z, y, x) cell-key order",
    );
}
