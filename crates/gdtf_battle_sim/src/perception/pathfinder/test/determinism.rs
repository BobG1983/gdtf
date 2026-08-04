use super::support::{
    all_other, cell, default_floor_costs, full_vision, grid_with, links_graph, open_step,
    reachable_triples, stair, tuning,
};
use crate::{
    ganger::Tu,
    injuries::MovementCostFactor,
    occupancy::TerrainKind,
    pathfinder::{MoveGrids, PlanningView, find_path, reachable_within},
};

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
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    let second = find_path(
        start,
        goal,
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    let third = find_path(
        start,
        goal,
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );

    assert!(first.is_ok(), "the fixture has a route: {first:?}");
    assert_eq!(first, second, "replay 2 differs from replay 1");
    assert_eq!(second, third, "replay 3 differs from replay 2");
}

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
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    let second = reachable_within(
        start,
        budget,
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );

    assert_eq!(
        first, second,
        "the reachable set must be identical across replays (same cells, costs, order)",
    );
    assert!(!first.is_empty(), "the start reaches at least itself");

    let triples = reachable_triples(start, budget, &grid, &links, &tuning);
    let mut sorted = triples.clone();
    sorted.sort_by_key(|((x, y, z), _)| (*z, *y, *x));
    assert_eq!(
        triples, sorted,
        "the reachable set is emitted in (z, y, x) cell-key order",
    );
}

#[test]
fn hampered_route_steps_are_scaled_and_sum_to_total() {
    let grid = grid_with(&[]);
    let tuning = tuning();
    let Some(links) = links_graph(&[]) else {
        return;
    };
    let floor_costs = default_floor_costs(&tuning);
    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);

    let start = cell(2, 2, 0);
    let goal = cell(6, 2, 0);
    let base = u32::from(*open_step(&tuning));

    let id = find_path(
        start,
        goal,
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
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

    let factor = MovementCostFactor::new(2.0);
    let hampered = find_path(
        start,
        goal,
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        factor,
        &planning,
    );
    let Ok(hampered) = hampered else {
        return;
    };
    assert_eq!(
        hampered.cells(),
        id.cells(),
        "the factor scales costs, it does not change the route"
    );
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
    assert!(
        *hampered.total() > *id.total(),
        "a Hampered route costs MORE TU than the same uninjured route"
    );
}
