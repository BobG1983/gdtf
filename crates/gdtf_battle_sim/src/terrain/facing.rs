//! Which way a placed terrain piece is turned.

use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

use crate::metric::Cell;

/// Cardinal facing of a placed terrain piece.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]
pub enum TerrainFacing {
    /// North (-Y).
    #[default]
    North,
    /// East (+X).
    East,
    /// South (+Y).
    South,
    /// West (-X).
    West,
}

impl TerrainFacing {
    /// All cardinals, in ring order.
    pub const ALL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];

    /// Integer cell delta for one step this way.
    #[must_use]
    pub const fn cell_step(self) -> Cell {
        match self {
            Self::North => Cell::new(0, -1),
            Self::East => Cell::new(1, 0),
            Self::South => Cell::new(0, 1),
            Self::West => Cell::new(-1, 0),
        }
    }
}
