//! Shared fixtures for the pathfinder tests — hand-built grids, a vertical-link
//! graph from authored slabs, and a default tuning. RELATIONS-ONLY: no pinned
//! shipped tunable magnitudes (the brittle-test rule) — costs are asserted by their
//! DERIVATION over the tuning the fixtures themselves expose.

use bevy::{platform::collections::HashSet, prelude::Entity};

use crate::{
    ganger::Tu,
    injuries::MovementCostFactor,
    metric::{Cell, CellLevel, Level, MAX_LEVELS},
    occupancy::{GRID_HEIGHT, GRID_WIDTH, OccupancyGrid, TerrainKind},
    pathfinder::{Path, PlanningView, find_path, reachable_within},
    terrain::floor::FloorCostGrid,
    test_support::{SituationBuilder, key},
    tuning::CombatTuning,
    vertical::{VerticalLink, VerticalLinkGraph, build_vertical_link_graph},
    visibility::{FactionRelation, SquadVisibility},
};

/// The central `(x, y, level)` cell-key helper, re-exported under a terse name for
/// the sibling test files (the vertical-test `key` precedent).
pub(super) fn cell(x: i32, y: i32, level: u8) -> CellLevel {
    key(x, y, level)
}

/// An occupant-faction resolver that maps EVERY occupant to
/// [`FactionRelation::Other`] — the default for the geometry fixtures (which carry no
/// occupants, so the resolver is never actually invoked).
pub(super) fn all_other(_occupant: Entity) -> FactionRelation {
    FactionRelation::Other
}

/// A [`SquadVisibility`] with the ENTIRE grid extent (`GRID_WIDTH × GRID_HEIGHT ×
/// MAX_LEVELS`) both VISIBLE and EXPLORED — the "full vision" fog the pre-GTW-353
/// geometry fixtures route under (every cell routable, so the visibility gate is a
/// no-op and the route depends only on geometry).
pub(super) fn full_vision() -> SquadVisibility {
    let mut all = HashSet::default();
    for level in 0..MAX_LEVELS {
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_possible_wrap,
                    reason = "x/y are 0..60 and level is 0..MAX_LEVELS (8) by the loop bounds, so \
                              the usize/u8 -> i32/u8 narrowing cannot truncate or wrap"
                )]
                let c = CellLevel::new(Cell::new(x as i32, y as i32), Level::new(level));
                all.insert(c);
            }
        }
    }
    SquadVisibility::new(all.clone(), all)
}

/// A hand-seeded [`SquadVisibility`] from explicit VISIBLE and EXPLORED cell lists —
/// the GTW-353 fixtures build the three states by hand (UNSEEN is whatever is in
/// neither list). The VISIBLE cells are also added to EXPLORED to honour the accrual
/// invariant `VISIBLE ⊆ EXPLORED`.
pub(super) fn fog(visible: &[CellLevel], explored_only: &[CellLevel]) -> SquadVisibility {
    let visible_set: HashSet<CellLevel> = visible.iter().copied().collect();
    let mut explored_set = visible_set.clone();
    explored_set.extend(explored_only.iter().copied());
    SquadVisibility::new(visible_set, explored_set)
}

/// A fresh all-[`TerrainKind::Open`] grid with the given `(cell, terrain)`
/// placements set — a HAND-BUILT fixture (C6), never the asset loader.
pub(super) fn grid_with(terrain: &[(CellLevel, TerrainKind)]) -> OccupancyGrid {
    let mut grid = OccupancyGrid::new();
    for &(at, kind) in terrain {
        grid.set_terrain(at, kind);
    }
    grid
}

/// An EMPTY vertical-link graph — for same-storey-only routes (no links).
pub(super) fn no_links() -> VerticalLinkGraph {
    VerticalLinkGraph::default()
}

/// Build a vertical-link graph from the given links, authoring every endpoint cell
/// as a slab so the build's dangling check passes. Returns the built graph or
/// `None` on a (test-author) validation failure — keeping the test free of
/// `unwrap`/`expect`/`panic` (all denied in tests too).
pub(super) fn links_graph(links: &[VerticalLink]) -> Option<VerticalLinkGraph> {
    let mut builder = SituationBuilder::new();
    for link in links {
        builder = builder.slab_at(link.from).slab_at(link.to);
        builder = builder.vertical_link(*link);
    }
    let situation = builder.build();
    build_vertical_link_graph(&situation).ok()
}

/// The default combat tuning — the flat link cost + other combat coefficients.
/// The tests assert RELATIONS over these values (read back off `tuning`), never
/// the shipped default magnitudes.
pub(super) fn tuning() -> CombatTuning {
    CombatTuning::default()
}

/// The default [`FloorCostGrid`] for the pathfinder tests — a uniform grid seeded
/// from the `move_costs.open` value in the DEFAULT [`CombatTuning`] (=4), so the
/// test arithmetic is unchanged: every Open cell costs `open`, every step is priced
/// exactly as the pre-GTW-396 tests expected. Tests that need a different cost build
/// their own grid with [`FloorCostGrid::new`].
pub(super) fn default_floor_costs(tuning: &CombatTuning) -> FloorCostGrid {
    FloorCostGrid::new(tuning.move_costs.open, [])
}

/// The orthogonal (open) step cost from the tuning — read back from the default
/// [`FloorCostGrid`] rather than from `tuning.move_costs.open` directly, so the
/// value tracks the LIVE cost source (GTW-396 Decision B / C1).
pub(super) fn open_step(tuning: &CombatTuning) -> Tu {
    Tu::new(*tuning.move_costs.open)
}

/// The flat vertical-link hop cost from the tuning — read back, not hard-coded.
pub(super) fn link_step(tuning: &CombatTuning) -> Tu {
    Tu::new(*tuning.link_tu)
}

/// Sum the per-step edge costs along a route by re-deriving each step from
/// `floor_costs` and `tuning` — the INDEPENDENT step-by-step total the §48
/// bit-identity checks the [`Path`]'s own `total` against.
///
/// Each consecutive cell pair is one edge: a same-storey orthogonal/diagonal terrain
/// step (priced via [`FloorCostGrid::cost`], GTW-396), or a cross-storey
/// vertical-link hop (flat `link_tu`). The sum is computed in a wide `u32` (a long
/// route can exceed a single `u8` step), the same width the search accumulates in.
pub(super) fn summed_step_cost(
    path: &Path,
    floor_costs: &FloorCostGrid,
    tuning: &CombatTuning,
) -> u32 {
    let cells = path.cells();
    let mut total = 0u32;
    for pair in cells.windows(2) {
        let [from, to] = pair else { continue };
        let step = step_cost_between(*from, *to, floor_costs, tuning);
        total += u32::from(*step);
    }
    total
}

/// The per-step edge cost between two CONSECUTIVE route cells — the same cost the
/// search relaxed with, re-derived independently for the §48 cross-check.
///
/// A same-storey pair is a floor step (orthogonal = `floor_costs.cost(&to)`,
/// diagonal = its octile, GTW-396); a different-storey pair is a vertical-link hop
/// (the flat `link_tu`). The destination's floor cost drives the terrain step.
pub(super) fn step_cost_between(
    from: CellLevel,
    to: CellLevel,
    floor_costs: &FloorCostGrid,
    tuning: &CombatTuning,
) -> Tu {
    if from.z != to.z {
        // A storey change is a vertical-link hop, priced at the flat link cost.
        return link_step(tuning);
    }
    let diagonal = from.x != to.x && from.y != to.y;
    // GTW-396: read from FloorCostGrid instead of tuning.move_costs (the live source).
    let move_cost = *floor_costs.cost(&to);
    if !diagonal {
        return Tu::new(move_cost);
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "move_cost is a small u8; move_cost * √2 rounds to a value that fits a u8 and is \
                  non-negative, so the cast cannot truncate or sign-flip"
    )]
    let octile = (f32::from(move_cost) * std::f32::consts::SQRT_2).round() as u8;
    Tu::new(octile)
}

/// Run `find_path` over a fixture under FULL VISION (the geometry fixtures' default
/// fog — every cell routable) and return the route, asserting it succeeded — keeps the
/// Ok-needing tests free of `unwrap`/`expect`. The GTW-353 visibility tests call
/// `find_path` directly with a hand-seeded [`PlanningView`].
pub(super) fn ok_path(
    start: CellLevel,
    goal: CellLevel,
    grid: &OccupancyGrid,
    links: &VerticalLinkGraph,
    tuning: &CombatTuning,
) -> Option<Path> {
    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    let floor_costs = default_floor_costs(tuning);
    let result = find_path(
        start,
        goal,
        grid,
        links,
        tuning,
        &floor_costs,
        MovementCostFactor::IDENTITY,
        &planning,
    );
    assert!(result.is_ok(), "expected a route, got {result:?}");
    result.ok()
}

/// Run `reachable_within` under FULL VISION and reduce the result to a
/// `Vec<((x, y, z), Tu)>` so the relations read clearly in assertions.
pub(super) fn reachable_triples(
    start: CellLevel,
    budget: Tu,
    grid: &OccupancyGrid,
    links: &VerticalLinkGraph,
    tuning: &CombatTuning,
) -> Vec<((i32, i32, i32), Tu)> {
    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    let floor_costs = default_floor_costs(tuning);
    reachable_within(
        start,
        budget,
        grid,
        links,
        tuning,
        &floor_costs,
        MovementCostFactor::IDENTITY,
        &planning,
    )
    .into_iter()
    .map(|(c, cost)| ((c.x, c.y, c.z), cost))
    .collect()
}

/// Reduce a `reachable_within` result to `Vec<((x, y, z), Tu)>` — the same projection
/// as [`reachable_triples`] but over a CALLER-supplied [`PlanningView`] (the GTW-353
/// fixtures seed their own fog + occupant resolver).
pub(super) fn reachable_triples_with<R>(
    start: CellLevel,
    budget: Tu,
    grid: &OccupancyGrid,
    links: &VerticalLinkGraph,
    tuning: &CombatTuning,
    planning: &PlanningView<'_, R>,
) -> Vec<((i32, i32, i32), Tu)>
where
    R: Fn(Entity) -> FactionRelation,
{
    let floor_costs = default_floor_costs(tuning);
    reachable_within(
        start,
        budget,
        grid,
        links,
        tuning,
        &floor_costs,
        MovementCostFactor::IDENTITY,
        planning,
    )
    .into_iter()
    .map(|(c, cost)| ((c.x, c.y, c.z), cost))
    .collect()
}

/// Whether a `(x, y, z)` cell appears in a reachable-triples set.
pub(super) fn reachable_contains(set: &[((i32, i32, i32), Tu)], want: (i32, i32, i32)) -> bool {
    set.iter().any(|(c, _)| *c == want)
}

/// A bidirectional stair link between two `(cell, level)` endpoints — the common
/// vertical fixture.
pub(super) fn stair(from: CellLevel, to: CellLevel) -> VerticalLink {
    use crate::vertical::LinkKind;
    VerticalLink::new(from, to, LinkKind::stair())
}
