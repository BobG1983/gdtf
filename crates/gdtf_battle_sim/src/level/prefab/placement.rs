//! One terrain piece placement inside a prefab.

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use crate::{
    metric::CellLevel,
    terrain::{def::TerrainUuid, facing::TerrainFacing},
};

/// Place a terrain piece at a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TypePath)]
pub struct TerrainPlacementEntry {
    /// Terrain definition to spawn.
    pub piece:  TerrainUuid,
    /// Cell where the piece is placed.
    pub at:     CellLevel,
    /// Which way the piece is turned.
    #[serde(default)]
    pub facing: TerrainFacing,
}

impl TerrainPlacementEntry {
    /// Build a placement entry.
    #[must_use]
    pub const fn new(piece: TerrainUuid, at: CellLevel, facing: TerrainFacing) -> Self {
        Self { piece, at, facing }
    }
}
