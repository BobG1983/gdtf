//! Cardinal facings on the wire.

use gdtf_battle_sim::terrain::facing::{TerrainCorner, TerrainFacing};
use serde::{Deserialize, Serialize};

/// A cardinal side, mirroring the sim's own facing with no wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum TerrainFacingNet {
    /// North.
    North,
    /// East.
    East,
    /// South.
    South,
    /// West.
    West,
}

impl TerrainFacingNet {
    /// Mirror the sim's own facing.
    pub(in crate::mcp) const fn from_facing(facing: TerrainFacing) -> Self {
        match facing {
            TerrainFacing::North => Self::North,
            TerrainFacing::East => Self::East,
            TerrainFacing::South => Self::South,
            TerrainFacing::West => Self::West,
        }
    }

    /// Read a client's facing back as the sim's own.
    pub(in crate::mcp) const fn to_facing(self) -> TerrainFacing {
        match self {
            Self::North => TerrainFacing::North,
            Self::East => TerrainFacing::East,
            Self::South => TerrainFacing::South,
            Self::West => TerrainFacing::West,
        }
    }
}

/// A diagonal corner, mirroring the sim's own corner with no wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum TerrainCornerNet {
    /// Where north meets east.
    NorthEast,
    /// Where south meets east.
    SouthEast,
    /// Where south meets west.
    SouthWest,
    /// Where north meets west.
    NorthWest,
}

impl TerrainCornerNet {
    /// Every corner a wall's turns are keyed by, for a case that walks them all.
    #[cfg(test)]
    pub(in crate::mcp) const ALL: [Self; 4] = [
        Self::NorthEast,
        Self::SouthEast,
        Self::SouthWest,
        Self::NorthWest,
    ];

    /// Mirror the sim's own corner.
    pub(in crate::mcp) const fn from_corner(corner: TerrainCorner) -> Self {
        match corner {
            TerrainCorner::NorthEast => Self::NorthEast,
            TerrainCorner::SouthEast => Self::SouthEast,
            TerrainCorner::SouthWest => Self::SouthWest,
            TerrainCorner::NorthWest => Self::NorthWest,
        }
    }

    /// Read a client's corner back as the sim's own.
    pub(in crate::mcp) const fn to_corner(self) -> TerrainCorner {
        match self {
            Self::NorthEast => TerrainCorner::NorthEast,
            Self::SouthEast => TerrainCorner::SouthEast,
            Self::SouthWest => TerrainCorner::SouthWest,
            Self::NorthWest => TerrainCorner::NorthWest,
        }
    }
}
