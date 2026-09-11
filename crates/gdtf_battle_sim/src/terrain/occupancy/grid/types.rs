//! Shared occupancy grid types and constants.

use bevy::prelude::{Deref, Entity};

use crate::{
    metric::{CellUnit, MAX_LEVELS},
    occupancy::TerrainKind,
};

/// Grid width in cells.
pub const GRID_WIDTH: usize = 60;

/// Grid height in cells.
pub const GRID_HEIGHT: usize = 60;

/// One dimension of the dense grid, in cells.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridExtent(usize);

impl GridExtent {
    /// Wrap a grid dimension in cells.
    #[must_use]
    pub const fn new(extent: usize) -> Self {
        Self(extent)
    }
}

/// The cell axis bound this extent reaches, saturating at [`i32::MAX`].
impl From<GridExtent> for CellUnit {
    fn from(extent: GridExtent) -> Self {
        Self::new(i32::try_from(*extent).unwrap_or(i32::MAX))
    }
}

/// Contents of one cell slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OccupancySlot {
    /// Terrain kind.
    pub terrain:  TerrainKind,
    /// Optional living occupant.
    pub occupant: Option<Entity>,
}

pub(super) const SLOT_COUNT: usize = GRID_WIDTH * GRID_HEIGHT * (MAX_LEVELS as usize);

/// Whether terrain blocks the cell.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Blocked(bool);

impl Blocked {
    /// Wrap a blocked flag.
    #[must_use]
    pub const fn new(blocked: bool) -> Self {
        Self(blocked)
    }
}

/// Whether pathfinding is blocked at this cell.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PathBlocked(bool);

impl PathBlocked {
    /// Wrap a path-blocked flag.
    #[must_use]
    pub const fn new(blocked: bool) -> Self {
        Self(blocked)
    }
}

/// Whether the cell occludes vision.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OccludesVision(bool);

impl OccludesVision {
    /// Wrap an occludes flag.
    #[must_use]
    pub const fn new(occludes: bool) -> Self {
        Self(occludes)
    }
}
