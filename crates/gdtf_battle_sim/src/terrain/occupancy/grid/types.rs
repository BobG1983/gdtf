//! Shared occupancy grid types and constants.

use bevy::{
    platform::collections::HashSet,
    prelude::{Deref, Entity},
};

use crate::{
    metric::{CellLevel, MAX_LEVELS},
    occupancy::TerrainKind,
};

/// Grid width in cells.
pub const GRID_WIDTH: usize = 60;

/// Grid height in cells.
pub const GRID_HEIGHT: usize = 60;

/// Contents of one cell slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OccupancySlot {
    /// Terrain kind.
    pub terrain: TerrainKind,
    /// Optional living occupant.
    pub occupant: Option<Entity>,
}

pub(super) const SLOT_COUNT: usize = GRID_WIDTH * GRID_HEIGHT * (MAX_LEVELS as usize);

/// Set of cells whose cover has been destroyed.
#[derive(Deref, Debug, Clone, Default, PartialEq, Eq)]
pub struct DestroyedCover(HashSet<CellLevel>);

impl DestroyedCover {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a cell's cover as destroyed.
    pub fn mark(&mut self, cell_level: CellLevel) {
        self.0.insert(cell_level);
    }
}

/// Whether terrain blocks the cell.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Blocked(bool);

impl Blocked {
    #[must_use]
    pub const fn new(blocked: bool) -> Self {
        Self(blocked)
    }
}

/// Whether pathfinding is blocked at this cell.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PathBlocked(bool);

impl PathBlocked {
    #[must_use]
    pub const fn new(blocked: bool) -> Self {
        Self(blocked)
    }
}

/// Whether the cell occludes vision.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OccludesVision(bool);

impl OccludesVision {
    #[must_use]
    pub const fn new(occludes: bool) -> Self {
        Self(occludes)
    }
}

/// Whether cover at this cell has been destroyed.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverDestroyed(bool);

impl CoverDestroyed {
    #[must_use]
    pub const fn new(destroyed: bool) -> Self {
        Self(destroyed)
    }
}
