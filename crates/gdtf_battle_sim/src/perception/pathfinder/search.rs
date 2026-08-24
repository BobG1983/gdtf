//! Public pathfinding entry points: find a path and list reachable cells.

use bevy::prelude::Entity;

use super::{
    core::{SearchGrids, StopRule, relax},
    departure::Departure,
    grids::MoveGrids,
    path::{Path, PathBlocked, PathCost},
    planning::PlanningView,
};
use crate::{
    acts::DismountSurcharge, ganger::Tu, injuries::MovementCostFactor, metric::CellLevel,
    tuning::MoveCost, visibility::FactionRelation,
};

/// Minimum per-step move cost used by the heuristic.
pub const MIN_MOVE_COST: MoveCost = MoveCost::new(4);

fn chebyshev_heuristic(from: CellLevel, goal: CellLevel) -> PathCost {
    let dx = (from.x - goal.x).unsigned_abs();
    let dy = (from.y - goal.y).unsigned_abs();
    let steps = dx.max(dy);
    PathCost::new(steps * u32::from(*MIN_MOVE_COST))
}

/// A* path from `start` to `goal` on the occupancy + vertical-link graph.
///
/// # Errors
///
/// Returns [`PathBlocked`] when no route reaches `goal` under the mover's costs and planning rules.
pub fn find_path<R>(
    start: CellLevel,
    goal: CellLevel,
    terrain: MoveGrids<'_>,
    factor: MovementCostFactor,
    planning: &PlanningView<'_, R>,
) -> Result<Path, PathBlocked>
where
    R: Fn(Entity) -> FactionRelation,
{
    find_path_leaving(&Departure::anywhere(start), goal, terrain, factor, planning)
}

/// A* path from `departure`'s start to `goal`, leaving the start only by the cells it admits.
///
/// # Errors
///
/// Returns [`PathBlocked`] when no admitted departure reaches `goal` under the mover's rules.
pub fn find_path_leaving<R>(
    departure: &Departure,
    goal: CellLevel,
    terrain: MoveGrids<'_>,
    factor: MovementCostFactor,
    planning: &PlanningView<'_, R>,
) -> Result<Path, PathBlocked>
where
    R: Fn(Entity) -> FactionRelation,
{
    let start = departure.start();
    let grids = SearchGrids {
        grid: terrain.occupancy,
        links: terrain.links,
        tuning: terrain.tuning,
        floor_costs: terrain.floor_costs,
        factor,
        planning,
        departure,
    };
    let field = relax(
        start,
        grids,
        |cell| chebyshev_heuristic(cell, goal),
        |cell, _cost| {
            if cell == goal {
                StopRule::Done
            } else {
                StopRule::Expand
            }
        },
    );

    let Some(cells) = field.reconstruct(goal) else {
        return Err(PathBlocked);
    };
    let steps = field.step_costs(&cells);
    let total = field.cost_of(&goal).unwrap_or(PathCost::ZERO).to_tu();
    Ok(Path::new(cells, steps, total))
}

// What a cell is quoted at: the route to it plus the exit act a walk off a seat also pays.
fn quoted(route: PathCost, exit: PathCost) -> PathCost {
    PathCost::new((*route).saturating_add(*exit))
}

/// Cells reachable from `departure`'s start within the TU budget, each quoted at its route
/// plus `surcharge`, leaving the start only by the cells `departure` admits.
#[must_use]
pub fn reachable_within<R>(
    departure: &Departure,
    budget: Tu,
    surcharge: DismountSurcharge,
    terrain: MoveGrids<'_>,
    factor: MovementCostFactor,
    planning: &PlanningView<'_, R>,
) -> Vec<(CellLevel, Tu)>
where
    R: Fn(Entity) -> FactionRelation,
{
    let start = departure.start();
    let grids = SearchGrids {
        grid: terrain.occupancy,
        links: terrain.links,
        tuning: terrain.tuning,
        floor_costs: terrain.floor_costs,
        factor,
        planning,
        departure,
    };
    let budget_cost = PathCost::new(u32::from(*budget));
    let exit = PathCost::new(u32::from(**surcharge));
    let field = relax(
        start,
        grids,
        |_cell| PathCost::ZERO,
        move |_cell, cost| {
            if quoted(cost, exit) > budget_cost {
                StopRule::Prune
            } else {
                StopRule::Expand
            }
        },
    );

    field
        .settled_sorted()
        .into_iter()
        .map(|(cell, cost)| (cell, quoted(cost, exit)))
        .filter(|(_, quote)| *quote <= budget_cost)
        .map(|(cell, quote)| (cell, quote.to_tu()))
        .collect()
}
