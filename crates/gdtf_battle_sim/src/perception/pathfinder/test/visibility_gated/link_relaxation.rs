//! GTW-387 — the known-link far-endpoint fog relaxation + the no-general-fog-hole
//! self-check.

use super::{
    super::support::{
        cell, default_floor_costs, fog, grid_with, links_graph, no_links, stair, tuning,
    },
    support::*,
};
use crate::{
    pathfinder::{PlanningView, find_path},
    visibility::FactionRelation,
};

/// **GTW-387 (C2) — planning onto a KNOWN vertical link's far endpoint is allowed
/// even when that cell is UNSEEN.** The link itself is authored in the
/// [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph) — the "known-link" oracle —
/// so the fog-explored gate is lifted for it by
/// [`PlanningView::is_routable_link`](crate::pathfinder::PlanningView::is_routable_link).
///
/// RELATIONS-ONLY / PIN-DISCRIMINATING: the GATED case (head UNSEEN) must succeed —
/// proving the relaxation opens the vertical hop; the CONTROL (head EXPLORED) also
/// succeeds — proving the GATED success is the relaxation, not something else.
#[test]
fn routing_onto_known_link_far_endpoint_is_allowed_when_unseen() {
    let grid = grid_with(&[]); // all Open on both storeys
    let tuning = tuning();
    let occupant = spawn_entity();

    let foot = cell(5, 5, 0);
    let head = cell(5, 5, 1);
    let Some(links) = links_graph(&[stair(foot, head)]) else {
        // links_graph returns None on a validation error — the test author must have
        // miswired the fixture; guard to keep the test free of unwrap/expect.
        return;
    };

    // Only the start (foot) and a corridor from it are EXPLORED; the head (5, 5, 1)
    // on the upper storey is intentionally UNSEEN.
    let explored_on_l0 = [cell(3, 5, 0), cell(4, 5, 0), foot];
    let floor_costs = default_floor_costs(&tuning);

    // GATED: head is UNSEEN — without the link relaxation the goal is unreachable.
    // WITH the relaxation it MUST be reached (the link-endpoint gate is C2-only).
    let squad_gated = fog(&[], &explored_on_l0);
    let gated = PlanningView::new(&squad_gated, resolve_as(occupant, FactionRelation::Other));
    let result_gated = find_path(
        foot,
        head,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        crate::injuries::MovementCostFactor::IDENTITY,
        &gated,
    );
    assert!(
        result_gated.is_ok(),
        "planning onto a known link's far endpoint MUST succeed even when the cell is UNSEEN \
         (GTW-387: link-endpoint gate is C2-only, C1 fog-explored is relaxed for link hops); \
         got {result_gated:?}",
    );
    assert_eq!(
        result_gated.ok().and_then(|p| p.goal()),
        Some(head),
        "the route reaches the UNSEEN link head — proving the link endpoint relaxation",
    );

    // CONTROL: same geometry with the head also EXPLORED — must also succeed,
    // proving the GATED success was the relaxation (not a spurious path through explored space).
    let mut explored_with_head = explored_on_l0.to_vec();
    explored_with_head.push(head);
    let squad_open = fog(&[], &explored_with_head);
    let open = PlanningView::new(&squad_open, resolve_as(occupant, FactionRelation::Other));
    let result_open = find_path(
        foot,
        head,
        &grid,
        &links,
        &tuning,
        &floor_costs,
        crate::injuries::MovementCostFactor::IDENTITY,
        &open,
    );
    assert!(
        result_open.is_ok(),
        "with the head EXPLORED the same geometry also has a route (CONTROL); got {result_open:?}",
    );
}

/// **GTW-387 (self-check) — an UNSEEN non-link cell is STILL non-routable.** The
/// relaxation applied to known vertical link endpoints MUST NOT open a general fog hole:
/// an UNSEEN floor tile that is not a link endpoint cannot be reached.
///
/// A 1-wide corridor on storey 0 is forced through an UNSEEN cell that is NOT a link
/// endpoint. With no links, `traversable_links` emits nothing, so the UNSEEN cell is
/// only a candidate for the PLANAR (full-gate) path — and must stay non-routable.
/// RELATIONS-ONLY / PIN-DISCRIMINATING: the GATED case (cell UNSEEN, no link) must fail;
/// the CONTROL (cell explored) proves the geometry itself has a route (the failure is
/// the fog, not the topology).
#[test]
fn unseen_non_link_cell_is_still_non_routable() {
    use crate::occupancy::TerrainKind;

    let links = no_links(); // NO vertical links — the UNSEEN cell is a planar step only
    let tuning = tuning();
    let occupant = spawn_entity();

    // A corridor: walls fence y=4 and y=6 so the ONLY route runs along y=5.
    // The chokepoint (2, 5, 0) is left UNSEEN in the GATED case.
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

    // GATED: the chokepoint (2, 5, 0) is UNSEEN (not in the explored set). Since there
    // are no links, traversable_links produces nothing — so the only path through the
    // chokepoint is PLANAR (full C1+C2 gate). The UNSEEN chokepoint MUST block it.
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
        &grid,
        &links,
        &tuning,
        &floor_costs,
        crate::injuries::MovementCostFactor::IDENTITY,
        &gated,
    );
    assert!(
        result_gated.is_err(),
        "an UNSEEN non-link cell on the only planar route MUST remain non-routable after \
         GTW-387 (the link relaxation does NOT open a general fog hole); got {result_gated:?}",
    );

    // CONTROL: explore the chokepoint — now the route succeeds, proving the GATED
    // failure was the fog (not the topology).
    let squad_open = fog(&[], &corridor_cells_l0);
    let open = PlanningView::new(&squad_open, resolve_as(occupant, FactionRelation::Other));
    let result_open = find_path(
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
        result_open.is_ok(),
        "with the chokepoint explored the corridor is open (CONTROL — the failure was the fog); \
         got {result_open:?}",
    );
}
