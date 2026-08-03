//! positions that are the **LOWER endpoint** of an authored stair link, and therefore
use bevy::{platform::collections::HashSet, prelude::Resource};

use crate::metric::CellLevel;

#[derive(Resource, Debug, Clone, Default)]
pub struct BraceStairCells(HashSet<CellLevel>);

impl BraceStairCells {
                        #[must_use]
    pub const fn new(cells: HashSet<CellLevel>) -> Self {
        Self(cells)
    }

            #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

                            #[must_use]
    pub fn contains(&self, cell_level: &CellLevel) -> bool {
        self.0.contains(cell_level)
    }
}
