//! C5 (GTW-353, the user-ratified OQ-5 ruling) — the **visibility-gated planning**
//! tests: the search routes only through ROUTABLE cells (UNSEEN non-routable, EXPLORED
//! routable), and within routable cells applies the visibility-aware blocking
//! predicate (own-squad always blocks, an enemy blocks iff squad-VISIBLE, walls /
//! cover scatter always block).
//!
//! RELATIONS-ONLY, pin-DISCRIMINATING (each test flips a verdict by changing ONLY the
//! fog / occupant under test, never a magnitude): every fixture pairs the gated case
//! against a control so the visibility gate — not the geometry — is proven the cause.
//! All fog is hand-seeded; no shipped tunable magnitude is asserted.

use bevy::prelude::{Entity, World};

use super::support::{
    cell, default_floor_costs, fog, grid_with, links_graph, no_links, reachable_triples_with,
    stair, tuning,
};
use crate::{
    metric::CellLevel,
    occupancy::{OccupancyGrid, TerrainKind},
    pathfinder::{PlanningView, find_path},
    visibility::FactionRelation,
};

/// A throwaway live [`Entity`] handle for an occupant fixture (a ganger standing in a
/// cell) — keeps the test free of `unwrap`/`expect`.
fn spawn_entity() -> Entity {
    World::new().spawn_empty().id()
}

/// A resolver mapping the single occupant `entity` to `relation`, anything else to
/// [`FactionRelation::Other`] — the GTW-353 occupant-faction seam, hand-seeded.
fn resolve_as(entity: Entity, relation: FactionRelation) -> impl Fn(Entity) -> FactionRelation {
    move |e| {
        if e == entity {
            relation
        } else {
            FactionRelation::Other
        }
    }
}

/// A 1-wide horizontal corridor on `y = 5`, `x = 0..=4` on storey 0: walls fence the
/// `y = 4` and `y = 6` rows so the ONLY route from `(0, 5)` to `(4, 5)` runs straight
/// along the corridor (no diagonal escape past the fence). Returns the grid.
fn corridor() -> OccupancyGrid {
    let mut walls = Vec::new();
    for x in 0..=4 {
        walls.push((cell(x, 4, 0), TerrainKind::Wall));
        walls.push((cell(x, 6, 0), TerrainKind::Wall));
    }
    grid_with(&walls)
}

/// The five corridor cells `(0..=4, 5, 0)` — the only walkable cells of the corridor.
fn corridor_cells() -> Vec<CellLevel> {
    (0..=4).map(|x| cell(x, 5, 0)).collect()
}

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
    let blocked = find_path(start, goal, &grid, &links, &tuning, &floor_costs, &gated);
    assert!(
        blocked.is_err(),
        "a route that can only reach the goal by crossing an UNSEEN cell is PathBlocked, got \
         {blocked:?}",
    );

    // CONTROL: with the chokepoint ALSO explored, the same geometry has a route —
    // proving the rejection above was the UNSEEN cell, not the walls.
    let squad_open = fog(&[], &corridor_cells());
    let open = PlanningView::new(&squad_open, resolve_as(occupant, FactionRelation::Other));
    let ok = find_path(start, goal, &grid, &links, &tuning, &floor_costs, &open);
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
    let result = find_path(start, goal, &grid, &links, &tuning, &floor_costs, &planning);
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

/// **C5 — a squad-VISIBLE enemy blocks the route.** The corridor forces the route
/// through `(2, 5, 0)`; an ENEMY standing there, on a cell that is squad-VISIBLE, is a
/// blocker — the goal is unreachable. The CONTROL (no occupant) proves the corridor is
/// otherwise open.
#[test]
fn visible_enemy_blocks_the_route() {
    let mut grid = corridor();
    let links = no_links();
    let tuning = tuning();
    let enemy = spawn_entity();
    let block_cell = cell(2, 5, 0);
    grid.set_occupant(block_cell, Some(enemy));
    let start = cell(0, 5, 0);
    let goal = cell(4, 5, 0);

    // The whole corridor is VISIBLE, and the enemy's cell is VISIBLE — so the enemy
    // blocks (is_ganger_visible(Other) is true on a visible cell).
    let floor_costs = default_floor_costs(&tuning);
    let squad = fog(&corridor_cells(), &[]);
    let planning = PlanningView::new(&squad, resolve_as(enemy, FactionRelation::Other));
    let result = find_path(start, goal, &grid, &links, &tuning, &floor_costs, &planning);
    assert!(
        result.is_err(),
        "a squad-VISIBLE enemy on the only route blocks it (PathBlocked), got {result:?}",
    );

    // CONTROL: a clean corridor with NO occupant — the same fog/geometry now has a
    // route, proving the block above was the visible enemy.
    let clear_grid = corridor();
    let open = find_path(
        start,
        goal,
        &clear_grid,
        &links,
        &tuning,
        &floor_costs,
        &planning,
    );
    assert!(
        open.is_ok(),
        "with no occupant the corridor is open — the block was the visible enemy, got {open:?}",
    );
}

/// **C5 — an INVISIBLE (not squad-VISIBLE) enemy does NOT block.** The SAME enemy on the
/// SAME chokepoint, but now its cell is only EXPLORED (not currently VISIBLE): the route
/// passes straight through it (bending around an unseen body would leak its position).
/// This flips the verdict of [`visible_enemy_blocks_the_route`] by changing ONLY the
/// enemy's cell visibility (VISIBLE → EXPLORED-only).
#[test]
fn invisible_enemy_does_not_block() {
    let mut grid = corridor();
    let links = no_links();
    let tuning = tuning();
    let enemy = spawn_entity();
    let block_cell = cell(2, 5, 0);
    grid.set_occupant(block_cell, Some(enemy));
    let start = cell(0, 5, 0);
    let goal = cell(4, 5, 0);

    // The corridor is EXPLORED (routable) but NOT currently VISIBLE — so the enemy on
    // it is not squad-VISIBLE and does NOT block (is_ganger_visible(Other) is false).
    let floor_costs = default_floor_costs(&tuning);
    let squad = fog(&[], &corridor_cells());
    let planning = PlanningView::new(&squad, resolve_as(enemy, FactionRelation::Other));
    let result = find_path(start, goal, &grid, &links, &tuning, &floor_costs, &planning);
    assert!(
        result.is_ok(),
        "an enemy the squad cannot see must NOT block the route, got {result:?}",
    );
    assert!(
        result.is_ok_and(|p| p.cells().contains(&block_cell)),
        "the route passes THROUGH the invisible enemy's cell (no fog-leaking detour)",
    );
}

/// **C5 — an OWN-SQUAD ganger ALWAYS blocks**, even on a merely-EXPLORED cell. The same
/// chokepoint and EXPLORED-only fog as [`invisible_enemy_does_not_block`] — but with the
/// occupant resolved as [`FactionRelation::OwnSquad`] the route is blocked. This flips
/// that test's verdict by changing ONLY the occupant's faction relation.
#[test]
fn own_squad_ganger_always_blocks() {
    let mut grid = corridor();
    let links = no_links();
    let tuning = tuning();
    let mate = spawn_entity();
    let block_cell = cell(2, 5, 0);
    grid.set_occupant(block_cell, Some(mate));
    let start = cell(0, 5, 0);
    let goal = cell(4, 5, 0);

    // Same EXPLORED-only fog (the cell is NOT currently VISIBLE) — but an own-squad
    // ganger is trivially visible, so it blocks regardless.
    let floor_costs = default_floor_costs(&tuning);
    let squad = fog(&[], &corridor_cells());
    let planning = PlanningView::new(&squad, resolve_as(mate, FactionRelation::OwnSquad));
    let result = find_path(start, goal, &grid, &links, &tuning, &floor_costs, &planning);
    assert!(
        result.is_err(),
        "an own-squad ganger always blocks, even on an EXPLORED-only cell, got {result:?}",
    );
}

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
    let result_gated = find_path(foot, head, &grid, &links, &tuning, &floor_costs, &gated);
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
    let result_open = find_path(foot, head, &grid, &links, &tuning, &floor_costs, &open);
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
    let result_gated = find_path(start, goal, &grid, &links, &tuning, &floor_costs, &gated);
    assert!(
        result_gated.is_err(),
        "an UNSEEN non-link cell on the only planar route MUST remain non-routable after \
         GTW-387 (the link relaxation does NOT open a general fog hole); got {result_gated:?}",
    );

    // CONTROL: explore the chokepoint — now the route succeeds, proving the GATED
    // failure was the fog (not the topology).
    let squad_open = fog(&[], &corridor_cells_l0);
    let open = PlanningView::new(&squad_open, resolve_as(occupant, FactionRelation::Other));
    let result_open = find_path(start, goal, &grid, &links, &tuning, &floor_costs, &open);
    assert!(
        result_open.is_ok(),
        "with the chokepoint explored the corridor is open (CONTROL — the failure was the fog); \
         got {result_open:?}",
    );
}

/// **C5 — VISIBLE/EXPLORED scatter (Cover) blocks** (true geometry on a routable cell).
/// The chokepoint is a Cover tile (the model's blocking scatter/prop); on a routable
/// (EXPLORED) cell it blocks the route. The CONTROL (the same cell EXPLORED but Open)
/// proves the block is the scatter, not the visibility.
#[test]
fn explored_scatter_blocks_the_route() {
    let links = no_links();
    let tuning = tuning();
    let occupant = spawn_entity();
    let block_cell = cell(2, 5, 0);
    let start = cell(0, 5, 0);
    let goal = cell(4, 5, 0);

    // The corridor with the chokepoint a Cover scatter tile.
    let mut walls = Vec::new();
    for x in 0..=4 {
        walls.push((cell(x, 4, 0), TerrainKind::Wall));
        walls.push((cell(x, 6, 0), TerrainKind::Wall));
    }
    walls.push((block_cell, TerrainKind::Cover));
    let grid = grid_with(&walls);

    // The whole corridor is EXPLORED (routable); the Cover scatter still blocks.
    let floor_costs = default_floor_costs(&tuning);
    let squad = fog(&[], &corridor_cells());
    let planning = PlanningView::new(&squad, resolve_as(occupant, FactionRelation::Other));
    let result = find_path(start, goal, &grid, &links, &tuning, &floor_costs, &planning);
    assert!(
        result.is_err(),
        "a blocking scatter (Cover) on the only route blocks it even when explored, got \
         {result:?}",
    );

    // CONTROL: the same cell EXPLORED but OPEN (no scatter) — the route succeeds, so the
    // block was the scatter geometry, not the fog.
    let open_grid = corridor();
    let open = find_path(
        start,
        goal,
        &open_grid,
        &links,
        &tuning,
        &floor_costs,
        &planning,
    );
    assert!(
        open.is_ok(),
        "with the scatter removed the explored corridor is open — the block was the Cover, got \
         {open:?}",
    );
}
