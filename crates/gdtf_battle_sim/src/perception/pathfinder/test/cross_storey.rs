use super::support::{
    all_other, cell, default_floor_costs, full_vision, grid_with, link_step, links_graph, ok_path,
    stair, summed_step_cost, tuning,
};
use crate::pathfinder::PlanningView;

#[test]
fn route_traverses_vertical_link_and_accumulates_link_tu() {
    let grid = grid_with(&[]);
    let tuning = tuning();

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

    let mut storey_changes = 0;
    for pair in path.cells().windows(2) {
        let [from, to] = pair else { continue };
        if from.z != to.z {
            storey_changes += 1;
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

    assert!(
        *path.total() > *link_step(&tuning),
        "the total accumulates the link_tu hop PLUS the terrain steps",
    );

    let floor_costs = default_floor_costs(&tuning);
    let summed = summed_step_cost(&path, &floor_costs, &tuning);
    assert_eq!(
        u32::from(*path.total()),
        summed,
        "the cross-storey total equals the summed per-step costs (link + terrain)",
    );
}

#[test]
fn upper_storey_unreachable_without_a_link() {
    let grid = grid_with(&[]);
    let tuning = tuning();
    let links = super::support::no_links();

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
