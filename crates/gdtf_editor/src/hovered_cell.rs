//! Hovered canvas cell under the pointer.

use bevy::prelude::*;
use gdtf_battle_sim::{metric::Level, prelude::Cell};

/// Optional cell and level currently under the editor cursor.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct HoveredCell {
    slot: Option<(Cell, Level)>,
}

impl HoveredCell {
    /// Empty hover.
    #[must_use]
    pub const fn new() -> Self {
        Self { slot: None }
    }

    /// Hovered cell, if any.
    #[must_use]
    pub const fn cell(&self) -> Option<Cell> {
        match self.slot {
            Some((cell, _)) => Some(cell),
            None => None,
        }
    }

    /// Hovered level, if any.
    #[must_use]
    pub const fn level(&self) -> Option<Level> {
        match self.slot {
            Some((_, level)) => Some(level),
            None => None,
        }
    }

    /// Set the hovered slot.
    pub const fn set(&mut self, cell: Cell, level: Level) {
        self.slot = Some((cell, level));
    }

    /// Clear the hover.
    pub const fn clear(&mut self) {
        self.slot = None;
    }
}
