//! Input-side path preview resource written by the editor/input layer.

use bevy::prelude::*;
use gdtf_battle_sim::prelude::{CellLevel, Tu};

/// Planned walk path and total TU cost shown under the cursor.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct PathPreview {
    cells: Vec<CellLevel>,
    cost: Tu,
}

impl PathPreview {
    /// Build a path preview from cells and total cost.
    #[must_use]
    pub const fn new(cells: Vec<CellLevel>, cost: Tu) -> Self {
        Self { cells, cost }
    }

    /// Empty path with zero cost.
    #[must_use]
    pub const fn cleared() -> Self {
        Self {
            cells: Vec::new(),
            cost: Tu::new(0),
        }
    }

    /// Cells along the planned path.
    #[must_use]
    pub fn cells(&self) -> &[CellLevel] {
        &self.cells
    }

    /// Total TU cost of the path.
    #[must_use]
    pub const fn cost(&self) -> Tu {
        self.cost
    }

    /// Whether no cells are planned.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}
