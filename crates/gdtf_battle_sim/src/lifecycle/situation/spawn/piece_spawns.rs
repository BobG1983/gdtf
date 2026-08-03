//! The authored `(cell, key)` placement records: [`CoverSpawn`], [`SlabSpawn`],
//! [`FloorSpawn`], and the GTW-545 [`FieldSpawn`] — each an authored `(cell, level)`
use serde::{Deserialize, Serialize};

use crate::{effects::fields::FieldKey, metric::CellLevel, terrain::def::TerrainUuid};

/// One authored piece of cover — a wall *or* a scatter prop.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct CoverSpawn {
        pub at:    CellLevel,
                                pub piece: TerrainUuid,
}

impl CoverSpawn {
        #[must_use]
    pub const fn new(at: CellLevel, piece: TerrainUuid) -> Self {
        Self { at, piece }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct SlabSpawn {
        pub at:    CellLevel,
                                                pub piece: TerrainUuid,
}

impl SlabSpawn {
        #[must_use]
    pub const fn new(at: CellLevel, piece: TerrainUuid) -> Self {
        Self { at, piece }
    }
}

/// case — a uniform floor) is the cheapest authored state and the `#[serde(default)]`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct FloorSpawn {
        pub at:    CellLevel,
        pub piece: TerrainUuid,
}

impl FloorSpawn {
        #[must_use]
    pub const fn new(at: CellLevel, piece: TerrainUuid) -> Self {
        Self { at, piece }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct FieldSpawn {
        pub at:    CellLevel,
                                pub field: FieldKey,
}

impl FieldSpawn {
        #[must_use]
    pub const fn new(at: CellLevel, field: FieldKey) -> Self {
        Self { at, field }
    }
}
