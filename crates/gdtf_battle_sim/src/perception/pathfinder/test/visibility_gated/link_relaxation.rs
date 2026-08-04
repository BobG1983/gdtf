use super::{
    super::support::{
        cell, default_floor_costs, fog, grid_with, links_graph, no_links, stair, tuning,
    },
    support::*,
};
use crate::{
    pathfinder::{MoveGrids, PlanningView, find_path},
    visibility::FactionRelation,
};

#[test]
fn routing_onto_known_link_far_endpoint_is_allowed_when_unseen() {
    let grid = grid_with(&[]);
    let tuning = tuning();
    let occupant = spawn_entity();

    let foot = cell(5, 5, 0);
    let head = cell(5, 5, 1);
    let Some(links) = links_graph(&[stair(foot, head)]) else {
        return;
    };

    let explored_on_l0 = [cell(3, 5, 0), cell(4, 5, 0), foot];
    let floor_costs = default_floor_costs(&tuning);

    let squad_gated = fog(&[], &explored_on_l0);
    let gated = PlanningView::new(&squad_gated, resolve_as(occupant, FactionRelation::Other));
    let result_gated = find_path(
        foot,
        head,
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        crate::injuries::MovementCostFactor::IDENTITY,
        &gated,
    );
    assert!(
        result_gated.is_ok(),
        "planning onto a known link's far endpoint MUST succeed even when the cell is UNSEEN \
         ( link-endpoint gate is C2-only, C1 fog-explored is relaxed for link hops); \
         got {result_gated:?}",
    );
    assert_eq!(
        result_gated.ok().and_then(|p| p.goal()),
        Some(head),
        "the route reaches the UNSEEN link head — proving the link endpoint relaxation",
    );

    let mut explored_with_head = explored_on_l0.to_vec();
    explored_with_head.push(head);
    let squad_open = fog(&[], &explored_with_head);
    let open = PlanningView::new(&squad_open, resolve_as(occupant, FactionRelation::Other));
    let result_open = find_path(
        foot,
        head,
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        crate::injuries::MovementCostFactor::IDENTITY,
        &open,
    );
    assert!(
        result_open.is_ok(),
        "with the head EXPLORED the same geometry also has a route (CONTROL); got {result_open:?}",
    );
}

#[test]
fn unseen_non_link_cell_is_still_non_routable() {
    use crate::occupancy::TerrainKind;

    let links = no_links();
    let tuning = tuning();
    let occupant = spawn_entity();

    let mut walls = Vec::new();
    for x in 0..=4_i32 {
        walls.push((cell(x, 4, 0), TerrainKind::Wall));
        walls.push((cell(x, 6, 0), TerrainKind::Wall));
    }
    let grid = grid_with(&walls);

    let corridor_cells_l0: Vec<crate::metric::CellLevel> =
        (0..=4_i32).map(|x| cell(x, 5, 0)).collect();
    let start = cell(0, 5, 0);
    let goal = cell(4, 5, 0);
    let floor_costs = default_floor_costs(&tuning);

    let explored_without_chokepoint: Vec<crate::metric::CellLevel> = corridor_cells_l0
        .iter()
        .copied()
        .filter(|c| *c != cell(2, 5, 0))
        .collect();
    let squad_gated = fog(&[], &explored_without_chokepoint);
    let gated = PlanningView::new(&squad_gated, resolve_as(occupant, FactionRelation::Other));
    let result_gated = find_path(
        start,
        goal,
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        crate::injuries::MovementCostFactor::IDENTITY,
        &gated,
    );
    assert!(
        result_gated.is_err(),
        "an UNSEEN non-link cell on the only planar route MUST remain non-routable after \
          (the link relaxation does NOT open a general fog hole); got {result_gated:?}",
    );

    let squad_open = fog(&[], &corridor_cells_l0);
    let open = PlanningView::new(&squad_open, resolve_as(occupant, FactionRelation::Other));
    let result_open = find_path(
        start,
        goal,
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        crate::injuries::MovementCostFactor::IDENTITY,
        &open,
    );
    assert!(
        result_open.is_ok(),
        "with the chokepoint explored the corridor is open (CONTROL — the failure was the fog); \
         got {result_open:?}",
    );
}
