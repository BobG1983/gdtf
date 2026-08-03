//! Path and cover blocking queries on the occupancy grid.

use super::{
    storage::OccupancyGrid,
    types::{Blocked, CoverDestroyed, DestroyedCover, PathBlocked},
};
use crate::metric::CellLevel;

impl OccupancyGrid {
    /// Record that cover at this cell was destroyed.
    pub fn mark_cover_destroyed(&mut self, cell_level: CellLevel) {
        self.destroyed_cover.mark(cell_level);
    }

    /// Whether cover at this cell has been destroyed.
    #[must_use]
    pub fn is_cover_destroyed(&self, cell_level: &CellLevel) -> CoverDestroyed {
        CoverDestroyed::new(self.destroyed_cover.contains(cell_level))
    }

    /// Whether the cell is blocked by terrain (destroyed cover no longer blocks).
    #[must_use]
    pub fn is_blocked(&self, cell_level: &CellLevel) -> Blocked {
        if *self.is_cover_destroyed(cell_level) {
            return Blocked::new(false);
        }
        self.terrain(cell_level).blocks()
    }

    /// Read-only destroyed-cover set.
    #[must_use]
    pub const fn destroyed_cover(&self) -> &DestroyedCover {
        &self.destroyed_cover
    }

    /// Whether pathfinding is blocked at this cell.
    #[must_use]
    pub fn is_path_blocked(&self, cell_level: &CellLevel) -> PathBlocked {
        if *self.is_cover_destroyed(cell_level) {
            return PathBlocked::new(false);
        }
        PathBlocked::new(self.path_blocking.contains(cell_level))
    }

    /// Mark a cell as path-blocked.
    pub fn set_path_blocking(&mut self, cell_level: CellLevel) {
        self.path_blocking.insert(cell_level);
    }

    /// Clear path blocking for a cell.
    pub fn clear_path_blocking(&mut self, cell_level: CellLevel) {
        self.path_blocking.remove(&cell_level);
    }
}
