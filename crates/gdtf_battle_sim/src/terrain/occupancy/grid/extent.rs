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
            .filter_map(|(index, _)| Self::cell_level_of_index(SlotIndex::new(index)))
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
        (0..SLOT_COUNT).filter_map(|index| Self::cell_level_of_index(SlotIndex::new(index)))
    }

    // `None` for a slot index whose coordinates do not fit the cell types.
    fn cell_level_of_index(index: SlotIndex) -> Option<CellLevel> {
        let plane_size = GRID_WIDTH * GRID_HEIGHT;
        let level = u8::try_from(*index / plane_size).ok()?;
        let plane = *index % plane_size;
        let y = i32::try_from(plane / GRID_WIDTH).ok()?;
        let x = i32::try_from(plane % GRID_WIDTH).ok()?;
        Some(CellLevel::new(Cell::new(x, y), Level::new(level)))
    }
}
