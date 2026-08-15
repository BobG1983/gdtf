//! Authored (cell, key) placement records for terrain and fields.

use serde::{Deserialize, Serialize};

use crate::{
    effects::fields::FieldKey,
    metric::CellLevel,
    terrain::{def::TerrainUuid, facing::TerrainFacing},
};

/// One wall or scatter cover piece.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct CoverSpawn {
    /// Cell.
    pub at:     CellLevel,
    /// Terrain piece key.
    pub piece:  TerrainUuid,
    /// Which way the piece is turned.
    #[serde(default)]
    pub facing: TerrainFacing,
}

impl CoverSpawn {
    /// Build a cover spawn.
    #[must_use]
    pub const fn new(at: CellLevel, piece: TerrainUuid, facing: TerrainFacing) -> Self {
        Self { at, piece, facing }
    }
}

/// One slab placement.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct SlabSpawn {
    /// Cell.
    pub at:     CellLevel,
    /// Terrain piece key.
    pub piece:  TerrainUuid,
    /// Which way the piece is turned.
    #[serde(default)]
    pub facing: TerrainFacing,
}

impl SlabSpawn {
    /// Build a slab spawn.
    #[must_use]
    pub const fn new(at: CellLevel, piece: TerrainUuid, facing: TerrainFacing) -> Self {
        Self { at, piece, facing }
    }
}

/// One per-cell floor override.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct FloorSpawn {
    /// Cell.
    pub at:     CellLevel,
    /// Terrain piece key.
    pub piece:  TerrainUuid,
    /// Which way the piece is turned.
    #[serde(default)]
    pub facing: TerrainFacing,
}

impl FloorSpawn {
    /// Build a floor spawn.
    #[must_use]
    pub const fn new(at: CellLevel, piece: TerrainUuid, facing: TerrainFacing) -> Self {
        Self { at, piece, facing }
    }
}

/// One area-damage field placement.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct FieldSpawn {
    /// Cell.
    pub at:    CellLevel,
    /// Field key.
    pub field: FieldKey,
}

impl FieldSpawn {
    /// Build a field spawn.
    #[must_use]
    pub const fn new(at: CellLevel, field: FieldKey) -> Self {
        Self { at, field }
    }
}
