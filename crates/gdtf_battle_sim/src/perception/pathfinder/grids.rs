//! The grids a mover's route is costed against.

use crate::{
    occupancy::OccupancyGrid, terrain::floor::FloorCostGrid, tuning::CombatTuning,
    vertical::VerticalLinkGraph,
};

/// Occupancy, vertical links, floor costs, and the tuning that prices a step.
#[derive(Debug, Clone, Copy)]
pub struct MoveGrids<'a> {
    /// Who and what stands in each cell.
    pub occupancy:   &'a OccupancyGrid,
    /// Ladders, stairs, and other level-to-level links.
    pub links:       &'a VerticalLinkGraph,
    /// Per-cell floor movement cost.
    pub floor_costs: &'a FloorCostGrid,
    /// Combat tuning that prices each step.
    pub tuning:      &'a CombatTuning,
}
