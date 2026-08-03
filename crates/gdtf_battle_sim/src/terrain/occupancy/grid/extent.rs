//! Iterate authored, occupied, or all cells in the occupancy grid.

use super::{
    storage::{OccupancyGrid, SlotIndex},
    types::{GRID_HEIGHT, GRID_WIDTH, SLOT_COUNT},
};
use crate::{
    metric::{Cell, CellLevel, Level},
    occupancy::TerrainKind,
};

impl OccupancyGrid {
    /// Cells with non-Open terrain or an occupant.
    pub fn authored_or_occupied_cells(&self) -> impl Iterator<Item = CellLevel> + '_ {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.terrain != TerrainKind::Open || slot.occupant.is_some())
            .map(|(index, _)| Self::cell_level_of_index(SlotIndex::new(index)))
    }

    /// Lowest and highest authored storey, if any cells exist.
    #[must_use]
    pub fn authored_level_range(&self) -> Option<(Level, Level)> {
        self.authored_or_occupied_cells().fold(None, |range, key| {
            let level = key.level();
            Some(match range {
                None => (level, level),
                Some((lo, hi)) => (lo.min(level), hi.max(level)),
            })
        })
    }

    /// Every cell in the dense grid.
    pub fn all_cells(&self) -> impl Iterator<Item = CellLevel> {
        (0..SLOT_COUNT).map(|index| Self::cell_level_of_index(SlotIndex::new(index)))
    }

    fn cell_level_of_index(index: SlotIndex) -> CellLevel {
        let level = *index / (GRID_WIDTH * GRID_HEIGHT);
        let plane = *index % (GRID_WIDTH * GRID_HEIGHT);
        let y = plane / GRID_WIDTH;
        let x = plane % GRID_WIDTH;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            reason = "x/y are 0..60 and level is 0..MAX_LEVELS (8) by the index's own \
                      construction, so these conversions cannot truncate, wrap, or sign-flip"
        )]
        let key = (x as i32, y as i32, level as u8);
        CellLevel::new(Cell::new(key.0, key.1), Level::new(key.2))
    }
}
