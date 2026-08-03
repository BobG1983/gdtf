//! Internal A* / flood distance field used by find_path and reachable_within.

use std::{
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
};

use bevy::{platform::collections::HashMap, prelude::Entity};

use super::{path::PathCost, planning::PlanningView};
use crate::{
    ganger::Tu,
    injuries::MovementCostFactor,
    metric::CellLevel,
    occupancy::{OccupancyGrid, pathable_neighbors},
    terrain::floor::FloorCostGrid,
    tuning::CombatTuning,
    vertical::{VerticalLinkGraph, traversable_links},
    visibility::FactionRelation,
};

type CellKey = (i32, i32, i32);

fn cell_key(cell: CellLevel) -> CellKey {
    (cell.z, cell.y, cell.x)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrontierNode {
    priority: PathCost,
    cost: PathCost,
    cell: CellLevel,
}

impl Ord for FrontierNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| cell_key(self.cell).cmp(&cell_key(other.cell)))
    }
}

impl PartialOrd for FrontierNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Borrowed grids and planning view for one search.
pub(super) struct SearchGrids<'a, R>
where
    R: Fn(Entity) -> FactionRelation,
{
    pub(super) grid: &'a OccupancyGrid,
    pub(super) links: &'a VerticalLinkGraph,
    pub(super) tuning: &'a CombatTuning,
    pub(super) floor_costs: &'a FloorCostGrid,
    pub(super) factor: MovementCostFactor,
    pub(super) planning: &'a PlanningView<'a, R>,
}

impl<R> SearchGrids<'_, R>
where
    R: Fn(Entity) -> FactionRelation,
{
    fn edges(&self, origin: CellLevel) -> Vec<(CellLevel, Tu)> {
        let planar = pathable_neighbors(origin, self.grid, self.floor_costs, self.factor)
            .filter(|(neighbour, _)| *self.planning.is_routable(*neighbour, self.grid));
        let vertical = traversable_links(origin, self.links, self.tuning.link_tu)
            .filter(|(neighbour, _)| *self.planning.is_routable_link(*neighbour, self.grid));
        planar.chain(vertical).collect()
    }
}

/// Settled costs and predecessors from one relax pass.
pub(super) struct DistanceField {
    cost: HashMap<CellLevel, PathCost>,
    prev: HashMap<CellLevel, Option<CellLevel>>,
}

impl DistanceField {
    #[must_use]
    pub(super) fn cost_of(&self, cell: &CellLevel) -> Option<PathCost> {
        self.cost.get(cell).copied()
    }

    #[must_use]
    pub(super) fn settled_sorted(&self) -> Vec<(CellLevel, PathCost)> {
        let mut out: Vec<(CellLevel, PathCost)> = self
            .cost
            .iter()
            .map(|(&cell, &cost)| (cell, cost))
            .collect();
        out.sort_by_key(|(cell, _)| cell_key(*cell));
        out
    }

    #[must_use]
    pub(super) fn step_costs(&self, route: &[CellLevel]) -> Vec<Tu> {
        route
            .windows(2)
            .map(|window| {
                let from = self.cost_of(&window[0]).unwrap_or(PathCost::ZERO);
                let to = self.cost_of(&window[1]).unwrap_or(PathCost::ZERO);
                PathCost::new((*to).saturating_sub(*from)).to_tu()
            })
            .collect()
    }

    #[must_use]
    pub(super) fn reconstruct(&self, goal: CellLevel) -> Option<Vec<CellLevel>> {
        self.prev.get(&goal)?;
        let mut route = vec![goal];
        let mut current = goal;
        while let Some(Some(predecessor)) = self.prev.get(&current).copied() {
            route.push(predecessor);
            current = predecessor;
        }
        route.reverse();
        Some(route)
    }
}

/// Dijkstra / A* relaxation with a stop rule.
pub(super) fn relax<H, S, R>(
    start: CellLevel,
    grids: SearchGrids<'_, R>,
    heuristic: H,
    stop: S,
) -> DistanceField
where
    H: Fn(CellLevel) -> PathCost,
    S: Fn(CellLevel, PathCost) -> StopRule,
    R: Fn(Entity) -> FactionRelation,
{
    let mut field = DistanceField {
        cost: HashMap::default(),
        prev: HashMap::default(),
    };
    let mut frontier: BinaryHeap<Reverse<FrontierNode>> = BinaryHeap::new();

    field.cost.insert(start, PathCost::ZERO);
    field.prev.insert(start, None);
    frontier.push(Reverse(FrontierNode {
        priority: heuristic(start),
        cost: PathCost::ZERO,
        cell: start,
    }));

    while let Some(Reverse(node)) = frontier.pop() {
        if field.cost.get(&node.cell).copied() != Some(node.cost) {
            continue;
        }

        match stop(node.cell, node.cost) {
            StopRule::Done => break,
            StopRule::Prune => continue,
            StopRule::Expand => {}
        }

        for (neighbour, step) in grids.edges(node.cell) {
            let next_cost = node.cost.add_step(step);
            let improved = field
                .cost
                .get(&neighbour)
                .is_none_or(|existing| next_cost < *existing);
            if !improved {
                continue;
            }
            field.cost.insert(neighbour, next_cost);
            field.prev.insert(neighbour, Some(node.cell));
            frontier.push(Reverse(FrontierNode {
                priority: PathCost::new(*next_cost + *heuristic(neighbour)),
                cost: next_cost,
                cell: neighbour,
            }));
        }
    }

    field
}

/// Controls whether the frontier expands, prunes, or stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StopRule {
    /// Goal reached.
    Done,
    /// Beyond budget; skip expansion.
    Prune,
    /// Keep expanding.
    Expand,
}
