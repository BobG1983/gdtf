use super::{storage::OccupancyGrid, types::OccludesVision};
use crate::{cover::HeightBand, metric::CellLevel};

impl OccupancyGrid {
                                                                                #[must_use]
    pub fn vision_occluder_at(&self, cell_level: &CellLevel) -> Option<HeightBand> {
        if *self.is_cover_destroyed(cell_level) {
            return None;
        }
        self.vision_blocking.get(cell_level).copied()
    }

                                            #[must_use]
    pub fn occludes_vision(&self, cell_level: &CellLevel, test_band: HeightBand) -> OccludesVision {
        OccludesVision::new(self.vision_occluder_at(cell_level).is_some_and(|band| {
            crate::clearance::round_clears_occupant(test_band, band)
                == crate::clearance::Clearance::Impacts
        }))
    }

                                    pub fn set_vision_blocking(&mut self, cell_level: CellLevel, band: HeightBand) {
        self.vision_blocking.insert(cell_level, band);
    }

                                pub fn clear_vision_blocking(&mut self, cell_level: CellLevel) {
        self.vision_blocking.remove(&cell_level);
    }
}
