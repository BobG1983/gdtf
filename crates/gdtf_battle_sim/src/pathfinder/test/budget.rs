//! C6 — [`reachable_within`](crate::pathfinder::reachable_within) RESPECTS the
//! budget: a cell whose cheapest cost is within budget is INCLUDED, a cell just over
//! budget is EXCLUDED, and every yielded cell's cost is `≤ budget`.

use super::support::{cell, grid_with, no_links, reachable_contains, reachable_triples, tuning};
use crate::{ganger::Tu, occupancy::TerrainKind};

/// On an open grid, a 5-step orthogonal corridor cell sits at exactly `5 × open`
/// cost; a 6th cell sits one step over. With the budget set to the 5th cell's cost,
/// the 5th is INCLUDED (cost == budget) and the 6th is EXCLUDED (cost > budget).
#[test]
fn cell_at_budget_included_one_step_over_excluded() {
    let grid = grid_with(&[]); // all Open
    let links = no_links();
    let tuning = tuning();
    let start = cell(5, 5, 0);

    // The cheapest cost to a due-east cell N steps away is N × open (a straight
    // orthogonal run; any diagonal detour costs more). Read `open` off the tuning.
    let open = u16::from(*tuning.move_costs.open);
    let within = open * 5; // (10, 5, 0): 5 orthogonal steps east

    // Budget = exactly the within-cell's cost, so it is included (≤) and the
    // next cell — one step further, strictly dearer — is over.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "open is <= 8 in the default table, so 5 * open and 6 * open fit a u8 budget"
    )]
    let budget = Tu::new(within as u8);
    let set = reachable_triples(start, budget, &grid, &links, &tuning);

    assert!(
        reachable_contains(&set, (10, 5, 0)),
        "the cell at exactly the budget cost is reachable (cost == budget is within)",
    );
    assert!(
        !reachable_contains(&set, (11, 5, 0)),
        "the cell one step OVER budget is excluded",
    );
    // The included cell's recorded cost is exactly `within`, and the over cell's
    // cheapest cost would have been `over` (recomputed, not pinned to a magnitude).
    let within_cost = set.iter().find(|(c, _)| *c == (10, 5, 0)).map(|(_, c)| *c);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "within fits a u8 as established above"
    )]
    let within_tu = Tu::new(within as u8);
    assert_eq!(
        within_cost,
        Some(within_tu),
        "the cell's recorded cost is its cheapest orthogonal cost",
    );
}

/// EVERY cell in the reachable set has a cost `≤ budget` — the bound holds for the
/// whole set, not just the probed cells (no over-budget cell leaks through).
#[test]
fn every_reachable_cell_is_within_budget() {
    let grid = grid_with(&[(cell(7, 5, 0), TerrainKind::Wall)]); // a wall to vary costs
    let links = no_links();
    let tuning = tuning();
    let start = cell(5, 5, 0);
    let budget = Tu::new(16);

    let set = reachable_triples(start, budget, &grid, &links, &tuning);
    assert!(!set.is_empty(), "the start reaches at least itself");
    for (c, cost) in &set {
        assert!(
            **cost <= *budget,
            "every reachable cell's cost is within budget ({c:?} at {cost:?})",
        );
    }
    // The start itself is always in the set at cost 0.
    let start_cost = set.iter().find(|(c, _)| *c == (5, 5, 0)).map(|(_, c)| *c);
    assert_eq!(
        start_cost,
        Some(Tu::new(0)),
        "the start is reachable at cost 0"
    );
}

/// A zero budget reaches ONLY the start cell (cost 0) — the tightest budget.
#[test]
fn zero_budget_reaches_only_the_start() {
    let grid = grid_with(&[]);
    let links = no_links();
    let tuning = tuning();
    let start = cell(3, 3, 0);

    let set = reachable_triples(start, Tu::new(0), &grid, &links, &tuning);
    assert_eq!(
        set,
        vec![((3, 3, 0), Tu::new(0))],
        "a zero budget reaches only the start cell at cost 0",
    );
}
