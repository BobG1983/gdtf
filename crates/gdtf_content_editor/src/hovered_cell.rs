use bevy::prelude::*;
use gdtf_battle_sim::{metric::Level, prelude::Cell};

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct HoveredCell {
        slot: Option<(Cell, Level)>,
}

impl HoveredCell {
        #[must_use]
    pub const fn new() -> Self {
        Self { slot: None }
    }

        #[must_use]
    pub const fn cell(&self) -> Option<Cell> {
        match self.slot {
            Some((cell, _)) => Some(cell),
            None => None,
        }
    }

        #[must_use]
    pub const fn level(&self) -> Option<Level> {
        match self.slot {
            Some((_, level)) => Some(level),
            None => None,
        }
    }

            pub const fn set(&mut self, cell: Cell, level: Level) {
        self.slot = Some((cell, level));
    }

        pub const fn clear(&mut self) {
        self.slot = None;
    }
}
