//! graph from authored slabs, and a default tuning. RELATIONS-ONLY: no pinned
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

pub(super) fn cell(x: i32, y: i32, level: u8) -> CellLevel {
    key(x, y, level)
}

pub(super) fn all_other(_occupant: Entity) -> FactionRelation {
    FactionRelation::Other
}

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

pub(super) fn fog(visible: &[CellLevel], explored_only: &[CellLevel]) -> SquadVisibility {
    let visible_set: HashSet<CellLevel> = visible.iter().copied().collect();
    let mut explored_set = visible_set.clone();
    explored_set.extend(explored_only.iter().copied());
    SquadVisibility::new(visible_set, explored_set)
}

pub(super) fn grid_with(terrain: &[(CellLevel, TerrainKind)]) -> OccupancyGrid {
    let mut grid = OccupancyGrid::new();
    for &(at, kind) in terrain {
        grid.set_terrain(at, kind);
        if *kind.blocks() {
            grid.set_path_blocking(at);
        }
    }
    grid
}

pub(super) fn grid_with_path_blocking(cells: &[CellLevel]) -> OccupancyGrid {
    let mut grid = OccupancyGrid::new();
    for &cell in cells {
        grid.set_path_blocking(cell);
    }
    grid
}

pub(super) fn no_links() -> VerticalLinkGraph {
    VerticalLinkGraph::default()
}

pub(super) fn links_graph(links: &[VerticalLink]) -> Option<VerticalLinkGraph> {
    let mut builder = SituationBuilder::new();
    for link in links {
        builder = builder.slab_at(link.from).slab_at(link.to);
        builder = builder.vertical_link(*link);
    }
    let situation = builder.build();
    build_vertical_link_graph(&situation).ok()
}

pub(super) fn tuning() -> CombatTuning {
    CombatTuning::default()
}

pub(super) fn default_floor_costs(tuning: &CombatTuning) -> FloorCostGrid {
    FloorCostGrid::new(tuning.move_costs.open, [])
}

pub(super) fn open_step(tuning: &CombatTuning) -> Tu {
    Tu::new(*tuning.move_costs.open)
}

pub(super) fn link_step(tuning: &CombatTuning) -> Tu {
    Tu::new(*tuning.link_tu)
}

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

pub(super) fn step_cost_between(
    from: CellLevel,
    to: CellLevel,
    floor_costs: &FloorCostGrid,
    tuning: &CombatTuning,
) -> Tu {
    if from.z != to.z {
        return link_step(tuning);
    }
    let diagonal = from.x != to.x && from.y != to.y;
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

pub(super) fn reachable_contains(set: &[((i32, i32, i32), Tu)], want: (i32, i32, i32)) -> bool {
    set.iter().any(|(c, _)| *c == want)
}

pub(super) fn stair(from: CellLevel, to: CellLevel) -> VerticalLink {
    use crate::vertical::LinkKind;
    VerticalLink::new(from, to, LinkKind::stair())
}
