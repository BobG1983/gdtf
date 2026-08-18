//! Which way a placed terrain piece is turned.

use serde::{Deserialize, Serialize};

/// Cardinal facing of a placed terrain piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]
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
}
