use super::{
    super::support::{
        cell, default_floor_costs, fog, grid_with, no_links, reachable_triples_with, tuning,
    },
    support::*,
};
use crate::{
    metric::CellLevel,
    pathfinder::{PlanningView, find_path},
    visibility::FactionRelation,
};

#[test]
fn route_only_through_unseen_is_path_blocked() {
    let grid = corridor();
    let links = no_links();
    let tuning = tuning();
    let occupant = spawn_entity();
    let start = cell(0, 5, 0);
    let goal = cell(4, 5, 0);

    let routable: Vec<CellLevel> = corridor_cells()
        .into_iter()
        .filter(|c| *c != cell(2, 5, 0))
        .collect();
    let squad_gated = fog(&[], &routable);
    let gated = PlanningView::new(&squad_gated, resolve_as(occupant, FactionRelation::Other));
    let floor_costs = default_floor_costs(&tuning);
    let blocked = find_path(
        start,
        goal,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        crate::injuries::MovementCostFactor::IDENTITY,
        &gated,
    );
    assert!(
        blocked.is_err(),
        "a route that can only reach the goal by crossing an UNSEEN cell is PathBlocked, got \
         {blocked:?}",
    );

    let squad_open = fog(&[], &corridor_cells());
    let open = PlanningView::new(&squad_open, resolve_as(occupant, FactionRelation::Other));
    let ok = find_path(
        start,
        goal,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        crate::injuries::MovementCostFactor::IDENTITY,
        &open,
    );
    assert!(
        ok.is_ok(),
        "with the chokepoint explored the geometry has a route — the rejection was UNSEEN, got \
         {ok:?}",
    );
}

#[test]
fn route_through_explored_is_allowed() {
    let grid = corridor();
    let links = no_links();
    let tuning = tuning();
    let occupant = spawn_entity();
    let start = cell(0, 5, 0);
    let goal = cell(4, 5, 0);

    let floor_costs = default_floor_costs(&tuning);
    let squad = fog(&[], &corridor_cells());
    let planning = PlanningView::new(&squad, resolve_as(occupant, FactionRelation::Other));
    let result = find_path(
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
        result.is_ok(),
        "an EXPLORED-only route must be allowed (EXPLORED stays routable), got {result:?}",
    );
    assert_eq!(
        result.ok().and_then(|p| p.goal()),
        Some(goal),
        "the explored route reaches the goal",
    );
}

#[test]
fn reachable_within_excludes_unseen_cells() {
    let grid = grid_with(&[]); 
    let links = no_links();
    let tuning = tuning();
    let occupant = spawn_entity();
    let start = cell(5, 5, 0);
    let budget = crate::ganger::Tu::new(20);

    let unseen = cell(6, 5, 0);
    let seen = cell(7, 5, 0);

    let routable = [start, cell(5, 6, 0), cell(6, 6, 0), cell(7, 6, 0), seen];
    let squad_gated = fog(&[], &routable);
    let gated = PlanningView::new(&squad_gated, resolve_as(occupant, FactionRelation::Other));
    let set_gated = reachable_triples_with(start, budget, &grid, &links, &tuning, &gated);
    assert!(
        !set_gated
            .iter()
            .any(|((x, y, z), _)| (*x, *y, *z) == (6, 5, 0)),
        "an UNSEEN cell within geometric budget is NOT yielded by the flood",
    );

    let mut with_near = routable.to_vec();
    with_near.push(unseen);
    let squad_open = fog(&[], &with_near);
    let open = PlanningView::new(&squad_open, resolve_as(occupant, FactionRelation::Other));
    let set_open = reachable_triples_with(start, budget, &grid, &links, &tuning, &open);
    assert!(
        set_open
            .iter()
            .any(|((x, y, z), _)| (*x, *y, *z) == (6, 5, 0)),
        "once explored the same cell IS within budget — its earlier absence was the fog",
    );
}
