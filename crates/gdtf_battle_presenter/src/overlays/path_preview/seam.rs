use bevy::prelude::*;
use gdtf_battle_sim::prelude::{CellLevel, Tu};

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct PathPreview {
            cells: Vec<CellLevel>,
                cost:  Tu,
}

impl PathPreview {
            #[must_use]
    pub const fn new(cells: Vec<CellLevel>, cost: Tu) -> Self {
        Self { cells, cost }
    }

            #[must_use]
    pub const fn cleared() -> Self {
        Self {
            cells: Vec::new(),
            cost:  Tu::new(0),
        }
    }

        #[must_use]
    pub fn cells(&self) -> &[CellLevel] {
        &self.cells
    }

            #[must_use]
    pub const fn cost(&self) -> Tu {
        self.cost
    }

        #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}
