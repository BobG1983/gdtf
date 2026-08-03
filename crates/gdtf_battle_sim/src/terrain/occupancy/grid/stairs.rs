//! on authored stair tiles, plus dual-cell stair presence registration/teardown.

use bevy::prelude::Deref;

use super::storage::OccupancyGrid;
use crate::{
    cover::HeightBand,
    metric::{CellLevel, Level, MAX_LEVELS},
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Default)]
pub struct StairEyeOffset(f32);

impl StairEyeOffset {
                    #[must_use]
    pub const fn new(offset: f32) -> Self {
        Self(offset)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StairCell(bool);

impl StairCell {
        #[must_use]
    pub const fn new(is_stair: bool) -> Self {
        Self(is_stair)
    }
}

impl OccupancyGrid {
                                        pub fn mark_stair_cell(&mut self, cell_level: CellLevel) {
        self.stair_cells.insert(cell_level);
    }

                            #[must_use]
    pub fn is_stair_cell(&self, cell_level: &CellLevel) -> StairCell {
        StairCell::new(self.stair_cells.contains(cell_level))
    }

                                            #[must_use]
    pub fn stair_eye_offset_at(&self, cell_level: &CellLevel) -> StairEyeOffset {
        if self.stair_cells.contains(cell_level) {
            StairEyeOffset::new(0.5)
        } else {
            StairEyeOffset::new(0.0)
        }
    }

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
            Some(_) => {
                None
            }
        }
    }

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
