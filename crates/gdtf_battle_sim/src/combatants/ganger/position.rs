//! Grid position component.

use bevy::prelude::{Component, Deref};

use crate::metric::{Cell, CellLevel, Level};

/// Where the ganger stands (cell + level).
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position(CellLevel);

impl Position {
    /// Wrap a cell/level.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self(at)
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::new(CellLevel::new(Cell::new(0, 0), Level::new(0)))
    }
}
