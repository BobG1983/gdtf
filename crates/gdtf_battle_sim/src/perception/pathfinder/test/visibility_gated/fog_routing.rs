//! C5 — fog routability: UNSEEN severs the route, EXPLORED stays routable, and
//! the reachable flood excludes UNSEEN cells.

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

/// **C5 — a route reachable ONLY through an UNSEEN cell is `PathBlocked`.** The corridor
/// forces the route through `(2, 5, 0)`; with that cell UNSEEN (in neither the VISIBLE
/// nor the EXPLORED set), the goal is unreachable. The CONTROL (everything explored)
/// proves the geometry itself has a route — so the rejection is the visibility gate.
#[test]
fn route_only_through_unseen_is_path_blocked() {
    let grid = corridor();
    let links = no_links();
    let tuning = tuning();
    let occupant = spawn_entity();
    let start = cell(0, 5, 0);
    let goal = cell(4, 5, 0);

    // GATED: every corridor cell EXCEPT the chokepoint (2, 5, 0) is explored; the
    // chokepoint is UNSEEN, so the only route is severed.
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

    // CONTROL: with the chokepoint ALSO explored, the same geometry has a route —
    // proving the rejection above was the UNSEEN cell, not the walls.
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

/// **C5 — a route through EXPLORED (remembered) cells IS allowed** (the OQ-5
/// interpretation: EXPLORED stays routable, only UNSEEN is severed). The whole corridor
/// is EXPLORED-only (never currently VISIBLE) and the route still succeeds and reaches
/// the goal.
#[test]
fn route_through_explored_is_allowed() {
    let grid = corridor();
    let links = no_links();
    let tuning = tuning();
    let occupant = spawn_entity();
    let start = cell(0, 5, 0);
    let goal = cell(4, 5, 0);

    // The corridor is remembered (EXPLORED) but not currently VISIBLE — the XCOM model.
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

/// **C5 — `reachable_within` never yields an UNSEEN cell.** From an open-grid start,
/// when only an L-shaped subset of cells is EXPLORED the flood reaches exactly the
/// routable cells and EXCLUDES the UNSEEN ones (even though they are within budget by
/// pure geometry). The CONTROL (those cells explored) shows they ARE geometrically in
/// range — so their absence is the visibility gate.
#[test]
fn reachable_within_excludes_unseen_cells() {
    let grid = grid_with(&[]); // all open
    let links = no_links();
    let tuning = tuning();
    let occupant = spawn_entity();
    let start = cell(5, 5, 0);
    let budget = crate::ganger::Tu::new(20);

    // A near cell we leave UNSEEN, and a far cell we explore — both within budget by
    // geometry on an open grid.
    let unseen = cell(6, 5, 0);
    let seen = cell(7, 5, 0);

    // GATED: only the start, the path to `seen`, and `seen` itself are explored;
    // `unseen` is deliberately omitted.
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

    // CONTROL: explore the near cell too — now it IS reachable, proving it was the fog
    // (not the budget / geometry) that excluded it.
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
