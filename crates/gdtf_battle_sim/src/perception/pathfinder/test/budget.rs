use super::support::{cell, grid_with, no_links, reachable_contains, reachable_triples, tuning};
use crate::{ganger::Tu, occupancy::TerrainKind};

#[test]
fn cell_at_budget_included_one_step_over_excluded() {
    let grid = grid_with(&[]);
    let links = no_links();
    let tuning = tuning();
    let start = cell(5, 5, 0);

    let open = u16::from(*tuning.move_costs.open);
    let within = open * 5;

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
    let within_cost = set.iter().find(|(c, _)| *c == (10, 5, 0)).map(|(_, c)| *c);
    let within_tu = Tu::new(within as u8);
    assert_eq!(
        within_cost,
        Some(within_tu),
        "the cell's recorded cost is its cheapest orthogonal cost",
    );
}

#[test]
fn every_reachable_cell_is_within_budget() {
    let grid = grid_with(&[(cell(7, 5, 0), TerrainKind::Wall)]);
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
    let start_cost = set.iter().find(|(c, _)| *c == (5, 5, 0)).map(|(_, c)| *c);
    assert_eq!(
        start_cost,
        Some(Tu::new(0)),
        "the start is reachable at cost 0"
    );
}

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
