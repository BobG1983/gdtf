//! Placement inputs used when building or rebuilding the occupancy grid.

use bevy::prelude::Entity;

use crate::{cover::HeightBand, metric::CellLevel, occupancy::TerrainKind};

/// Authored terrain at a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainPlacement {
    /// Cell and level.
    pub at:      CellLevel,
    /// Terrain kind.
    pub terrain: TerrainKind,
}

impl TerrainPlacement {
    /// Build a terrain placement.
    #[must_use]
    pub const fn new(at: CellLevel, terrain: TerrainKind) -> Self {
        Self { at, terrain }
    }
}

/// Living occupant placed at a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OccupantPlacement {
    /// Cell and level.
    pub at:       CellLevel,
    /// Occupant entity.
    pub occupant: Entity,
    /// Height band of the occupant silhouette.
    pub band:     HeightBand,
}

impl OccupantPlacement {
    /// Build an occupant placement.
    #[must_use]
    pub const fn new(at: CellLevel, occupant: Entity, band: HeightBand) -> Self {
        Self { at, occupant, band }
    }
}

/// Batch of terrain and occupant placements for grid construction.
#[derive(Debug, Clone, Default)]
pub struct OccupancyInput {
    /// Terrain placements.
    pub terrain:   Vec<TerrainPlacement>,
    /// Occupant placements.
    pub occupants: Vec<OccupantPlacement>,
}

impl OccupancyInput {
    /// Empty input.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
