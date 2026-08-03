//! Vision occluder queries on the occupancy grid.

use super::{storage::OccupancyGrid, types::OccludesVision};
use crate::{cover::HeightBand, metric::CellLevel};

impl OccupancyGrid {
    /// Height band of the vision occluder at this cell, if any.
    #[must_use]
    pub fn vision_occluder_at(&self, cell_level: &CellLevel) -> Option<HeightBand> {
        if *self.is_cover_destroyed(cell_level) {
            return None;
        }
        self.vision_blocking.get(cell_level).copied()
    }

    /// Whether a round at `test_band` is blocked by the occluder here.
    #[must_use]
    pub fn occludes_vision(&self, cell_level: &CellLevel, test_band: HeightBand) -> OccludesVision {
        OccludesVision::new(self.vision_occluder_at(cell_level).is_some_and(|band| {
            crate::clearance::round_clears_occupant(test_band, band)
                == crate::clearance::Clearance::Impacts
        }))
    }

    /// Set the vision-blocking band for a cell.
    pub fn set_vision_blocking(&mut self, cell_level: CellLevel, band: HeightBand) {
        self.vision_blocking.insert(cell_level, band);
    }

    /// Clear vision blocking for a cell.
    pub fn clear_vision_blocking(&mut self, cell_level: CellLevel) {
        self.vision_blocking.remove(&cell_level);
    }
}
