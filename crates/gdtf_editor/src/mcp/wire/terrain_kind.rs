//! The terrain form's kind pick on the wire.

use serde::{Deserialize, Serialize};

use crate::terrain_form::TerrainKindChoice;

/// Which terrain kind the Terrain draft is on, with no wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum TerrainKindNet {
    /// Full wall.
    Wall,
    /// Partial cover.
    Cover,
    /// Floor slab.
    Slab,
    /// Weapon emplacement.
    Emplacement,
}

impl TerrainKindNet {
    /// Mirror the form's own kind pick.
    pub(in crate::mcp) const fn from_choice(choice: TerrainKindChoice) -> Self {
        match choice {
            TerrainKindChoice::Wall => Self::Wall,
            TerrainKindChoice::Cover => Self::Cover,
            TerrainKindChoice::Slab => Self::Slab,
            TerrainKindChoice::Emplacement => Self::Emplacement,
        }
    }

    /// Read a client's kind back as the form's own.
    pub(in crate::mcp) const fn to_choice(self) -> TerrainKindChoice {
        match self {
            Self::Wall => TerrainKindChoice::Wall,
            Self::Cover => TerrainKindChoice::Cover,
            Self::Slab => TerrainKindChoice::Slab,
            Self::Emplacement => TerrainKindChoice::Emplacement,
        }
    }
}
