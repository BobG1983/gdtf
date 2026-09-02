//! Path and cover blocking queries on the occupancy grid.

use super::{
    storage::OccupancyGrid,
    types::{Blocked, PathBlocked},
};
use crate::metric::CellLevel;

impl OccupancyGrid {
    /// Whether the cell is blocked by the terrain kind standing in it.
    #[must_use]
    pub fn is_blocked(&self, cell_level: &CellLevel) -> Blocked {
        self.terrain(cell_level).blocks()
    }

    /// Whether pathfinding is blocked at this cell.
    #[must_use]
    pub fn is_path_blocked(&self, cell_level: &CellLevel) -> PathBlocked {
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
