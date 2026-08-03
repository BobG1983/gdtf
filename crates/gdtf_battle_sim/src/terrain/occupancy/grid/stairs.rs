//! Stair cell markers, eye offset, and dual-cell stair presence.

use bevy::prelude::Deref;

use super::storage::OccupancyGrid;
use crate::{
    cover::HeightBand,
    metric::{CellLevel, Level, MAX_LEVELS},
};

/// Extra eye height when standing on a stair cell (0.5 storeys).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Default)]
pub struct StairEyeOffset(f32);

impl StairEyeOffset {
    /// Wrap an eye offset.
    #[must_use]
    pub const fn new(offset: f32) -> Self {
        Self(offset)
    }
}

/// Whether a cell is an authored stair tile.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StairCell(bool);

impl StairCell {
    /// Wrap a stair-cell flag.
    #[must_use]
    pub const fn new(is_stair: bool) -> Self {
        Self(is_stair)
    }
}

impl OccupancyGrid {
    /// Mark a cell as a stair endpoint.
    pub fn mark_stair_cell(&mut self, cell_level: CellLevel) {
        self.stair_cells.insert(cell_level);
    }

    /// Whether this cell is a stair.
    #[must_use]
    pub fn is_stair_cell(&self, cell_level: &CellLevel) -> StairCell {
        StairCell::new(self.stair_cells.contains(cell_level))
    }

    /// Eye height offset for LOS when standing on this cell.
    #[must_use]
    pub fn stair_eye_offset_at(&self, cell_level: &CellLevel) -> StairEyeOffset {
        if self.stair_cells.contains(cell_level) {
            StairEyeOffset::new(0.5)
        } else {
            StairEyeOffset::new(0.0)
        }
    }

    /// Place a ganger on a stair: occupies lower and (if free) upper cell.
    ///
    /// Returns the upper cell when dual-cell presence was registered.
    pub fn register_stair_presence(
        &mut self,
        lower: CellLevel,
        entity: bevy::prelude::Entity,
        lower_band: HeightBand,
    ) -> Option<CellLevel> {
        self.set_occupant(lower, Some(entity));
        self.set_occupant_band(lower, Some(lower_band));

        let upper = Self::upper_cell(&lower)?;

        match self.occupant(&upper) {
            None => {
                self.set_occupant(upper, Some(entity));
                self.set_occupant_band(upper, Some(HeightBand::Low));
                Some(upper)
            }
            Some(other) if other == entity => {
                self.set_occupant_band(upper, Some(HeightBand::Low));
                Some(upper)
            }
            Some(_) => None,
        }
    }

    /// Clear dual-cell presence on the upper cell if it still holds this entity.
    pub fn clear_stair_upper(&mut self, upper: CellLevel, entity: bevy::prelude::Entity) {
        if self.occupant(&upper) == Some(entity) {
            self.set_occupant(upper, None);
            self.set_occupant_band(upper, None);
        }
    }

    fn upper_cell(lower: &CellLevel) -> Option<CellLevel> {
        let next = (*lower.level()).checked_add(1)?;
        if next >= MAX_LEVELS {
            return None;
        }
        Some(CellLevel::new(lower.cell(), Level::new(next)))
    }
}
