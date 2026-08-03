use super::{
    storage::OccupancyGrid,
    types::{Blocked, CoverDestroyed, DestroyedCover, PathBlocked},
};
use crate::metric::CellLevel;

impl OccupancyGrid {
                                                    pub fn mark_cover_destroyed(&mut self, cell_level: CellLevel) {
        self.destroyed_cover.mark(cell_level);
    }

            #[must_use]
    pub fn is_cover_destroyed(&self, cell_level: &CellLevel) -> CoverDestroyed {
        CoverDestroyed::new(self.destroyed_cover.contains(cell_level))
    }

                                        #[must_use]
    pub fn is_blocked(&self, cell_level: &CellLevel) -> Blocked {
        if *self.is_cover_destroyed(cell_level) {
            return Blocked::new(false);
        }
        self.terrain(cell_level).blocks()
    }

                        #[must_use]
    pub const fn destroyed_cover(&self) -> &DestroyedCover {
        &self.destroyed_cover
    }

                                                                        #[must_use]
    pub fn is_path_blocked(&self, cell_level: &CellLevel) -> PathBlocked {
        if *self.is_cover_destroyed(cell_level) {
            return PathBlocked::new(false);
        }
        PathBlocked::new(self.path_blocking.contains(cell_level))
    }

                                    pub fn set_path_blocking(&mut self, cell_level: CellLevel) {
        self.path_blocking.insert(cell_level);
    }

                                pub fn clear_path_blocking(&mut self, cell_level: CellLevel) {
        self.path_blocking.remove(&cell_level);
    }
}
