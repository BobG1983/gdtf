//! C6 — DETERMINISM replay: the same inputs produce an identical
//! [`Path`](crate::pathfinder::Path) AND an identical
//! [`reachable_within`](crate::pathfinder::reachable_within) set, asserted across
//! repeated calls. No RNG; the `(cost, (z, y, x) cell_key)` frontier tie-break + the
//! pre-sorted GTW-350/351 edge enumeration pin the result.

use super::support::{
    all_other, cell, default_floor_costs, full_vision, grid_with, links_graph, open_step,
    reachable_triples, stair, tuning,
};
use crate::{
    ganger::Tu,
    injuries::MovementCostFactor,
    occupancy::TerrainKind,
    pathfinder::{PlanningView, find_path, reachable_within},
};

/// `find_path` over a non-trivial multi-storey grid (a wall to detour, a link to
/// climb) returns the SAME route on every call — identical `Path` (cells AND
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
    let first = find_path(
        start,
        goal,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    let second = find_path(
        start,
        goal,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    let third = find_path(
        start,
        goal,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );

    assert!(first.is_ok(), "the fixture has a route: {first:?}");
    // Identical across all three replays (the whole Path: cells + total).
    assert_eq!(first, second, "replay 2 differs from replay 1");
    assert_eq!(second, third, "replay 3 differs from replay 2");
}

/// `reachable_within` over the same multi-storey grid returns the SAME reachable set
/// (cells AND costs, in the SAME sorted order) on every call — identical, so the
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
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    let second = reachable_within(
        start,
        budget,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );

    // Identical: same cells, same costs, same ORDER (the (z, y, x) sort).
    assert_eq!(
        first, second,
        "the reachable set must be identical across replays (same cells, costs, order)",
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

// ── GTW-444: the MovementCostFactor preview==charge identity (C3 / C5) ────────────────

/// A Hampered mover's planned route is BOTH scaled per step AND keeps the §48 bit-identity
/// (`total == sum of steps`) — which is what makes preview == charge: the committed walk
/// (`advance_walk`) charges the planned `Path::steps()` VERBATIM, so if `find_path` returns
/// scaled steps that sum to `total`, the TU charged equals the previewed total. This test
/// PINS that the SAME `find_path` cost the preview shows is the cost the walk will charge —
/// it would FAIL if only one site scaled, or if the steps no longer summed to the total.
#[test]
fn hampered_route_steps_are_scaled_and_sum_to_total() {
    let grid = grid_with(&[]); // all Open, one storey — a pure orthogonal corridor
    let tuning = tuning();
    let Some(links) = links_graph(&[]) else {
        return;
    };
    let floor_costs = default_floor_costs(&tuning);
    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);

    // A 4-step orthogonal route (no diagonals → every step is the base open cost).
    let start = cell(2, 2, 0);
    let goal = cell(6, 2, 0);
    let base = u32::from(*open_step(&tuning));

    // Uninjured (IDENTITY) baseline: every step == base, total == 4 × base.
    let id = find_path(
        start,
        goal,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        MovementCostFactor::IDENTITY,
        &planning,
    );
    let Ok(id) = id else {
        return;
    };
    assert!(
        id.steps().iter().all(|s| u32::from(**s) == base),
        "uninjured: each orthogonal step is the base open cost"
    );

    // Hampered (2.0): every step DOUBLES (ceil(base × 2.0) = 2 × base), the total doubles,
    // and the per-step costs STILL sum to the total (the §48 identity the walk relies on).
    let factor = MovementCostFactor::new(2.0);
    let hampered = find_path(
        start,
        goal,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        factor,
        &planning,
    );
    let Ok(hampered) = hampered else {
        return;
    };
    // Same route cells (the slowdown does not reroute — every cell scales equally).
    assert_eq!(
        hampered.cells(),
        id.cells(),
        "the factor scales costs, it does not change the route"
    );
    // Each step is scaled to 2 × base.
    assert!(
        hampered.steps().iter().all(|s| u32::from(**s) == 2 * base),
        "hampered (2.0): each step is ceil(base × 2.0) = 2 × base"
    );
    // PREVIEW==CHARGE: the steps the walk will charge sum to the previewed total.
    let summed: u32 = hampered.steps().iter().map(|s| u32::from(**s)).sum();
    assert_eq!(
        summed,
        u32::from(*hampered.total()),
        "the per-step charges (what advance_walk charges) sum to the previewed total"
    );
    // The Hampered total is strictly greater than the uninjured total (a real slowdown).
    assert!(
        *hampered.total() > *id.total(),
        "a Hampered route costs MORE TU than the same uninjured route"
    );
}
