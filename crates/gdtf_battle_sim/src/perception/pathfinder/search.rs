//! Public pathfinding entry points: find a path and list reachable cells.

use bevy::prelude::Entity;

use super::{
    core::{SearchGrids, StopRule, relax},
    path::{Path, PathBlocked, PathCost},
    planning::PlanningView,
};
use crate::{
    ganger::Tu,
    injuries::MovementCostFactor,
    metric::CellLevel,
    occupancy::OccupancyGrid,
    terrain::floor::FloorCostGrid,
    tuning::{CombatTuning, MoveCost},
    vertical::VerticalLinkGraph,
    visibility::FactionRelation,
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
#[expect(
    clippy::too_many_arguments,
    reason = "grids, endpoints, and MovementCostFactor are separate inputs; SearchGrids is internal only"
)]
pub fn find_path<R>(
    start: CellLevel,
    goal: CellLevel,
    grid: &OccupancyGrid,
    links: &VerticalLinkGraph,
    tuning: &CombatTuning,
    floor_costs: &FloorCostGrid,
    factor: MovementCostFactor,
    planning: &PlanningView<'_, R>,
) -> Result<Path, PathBlocked>
where
    R: Fn(Entity) -> FactionRelation,
{
    let grids = SearchGrids {
        grid,
        links,
        tuning,
        floor_costs,
        factor,
        planning,
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

/// All cells reachable from `start` within the TU budget.
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "grids, start, budget, and MovementCostFactor are separate inputs"
)]
pub fn reachable_within<R>(
    start: CellLevel,
    budget: Tu,
    grid: &OccupancyGrid,
    links: &VerticalLinkGraph,
    tuning: &CombatTuning,
    floor_costs: &FloorCostGrid,
    factor: MovementCostFactor,
    planning: &PlanningView<'_, R>,
) -> Vec<(CellLevel, Tu)>
where
    R: Fn(Entity) -> FactionRelation,
{
    let grids = SearchGrids {
        grid,
        links,
        tuning,
        floor_costs,
        factor,
        planning,
    };
    let budget_cost = PathCost::new(u32::from(*budget));
    let field = relax(
        start,
        grids,
        |_cell| PathCost::ZERO,
        move |_cell, cost| {
            if cost > budget_cost {
                StopRule::Prune
            } else {
                StopRule::Expand
            }
        },
    );

    field
        .settled_sorted()
        .into_iter()
        .filter(|(_, cost)| *cost <= budget_cost)
        .map(|(cell, cost)| (cell, cost.to_tu()))
        .collect()
}
