//! Cardinal facings on the wire.

use gdtf_battle_sim::terrain::facing::TerrainFacing;
use serde::{Deserialize, Serialize};

/// A cardinal side, mirroring the sim's own facing with no wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum TerrainFacingNet {
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
    pub(in crate::net_qa) const fn from_facing(facing: TerrainFacing) -> Self {
        match facing {
            TerrainFacing::North => Self::North,
            TerrainFacing::East => Self::East,
            TerrainFacing::South => Self::South,
            TerrainFacing::West => Self::West,
        }
    }

    /// Read a client's facing back as the sim's own.
    pub(in crate::net_qa) const fn to_facing(self) -> TerrainFacing {
        match self {
            Self::North => TerrainFacing::North,
            Self::East => TerrainFacing::East,
            Self::South => TerrainFacing::South,
            Self::West => TerrainFacing::West,
        }
    }
}
