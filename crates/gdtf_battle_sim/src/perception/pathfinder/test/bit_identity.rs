use super::support::{
    cell, default_floor_costs, grid_with, links_graph, ok_path, stair, step_cost_between,
    summed_step_cost, tuning,
};
use crate::occupancy::TerrainKind;

#[test]
fn same_storey_total_equals_summed_steps() {
    let mut grid = grid_with(&[(cell(4, 3, 0), TerrainKind::Cover)]);
    grid.set_terrain(cell(4, 3, 0), TerrainKind::Open);
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
    assert!(*path.total() > 0, "a real route has a positive total");
    assert!(path.cells().len() > 2, "a real multi-step route");
}

#[test]
fn carried_per_step_costs_match_each_edge_and_sum_to_total() {
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

    assert_eq!(
        path.steps().len(),
        path.cells().len().saturating_sub(1),
        "steps() carries exactly one entry per edge (aligned to cells[1..])",
    );

    for (edge, pair) in path.steps().iter().zip(path.cells().windows(2)) {
        let [from, to] = pair else { continue };
        assert_eq!(
            *edge,
            step_cost_between(*from, *to, &floor_costs, &tuning),
            "each carried per-step cost equals the edge cost between its consecutive cells",
        );
    }

    let carried_sum: u32 = path.steps().iter().map(|step| u32::from(**step)).sum();
    assert_eq!(
        carried_sum,
        u32::from(*path.total()),
        "the carried per-step costs sum to the Path total bit-for-bit (§48)",
    );
    assert_eq!(
        carried_sum,
        summed_step_cost(&path, &floor_costs, &tuning),
        "the carried steps agree with the independent per-edge re-derivation",
    );
}
