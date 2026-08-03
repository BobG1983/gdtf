use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use crate::{metric::CellLevel, terrain::def::TerrainUuid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TypePath)]
pub struct TerrainPlacementEntry {
            pub piece: TerrainUuid,
        pub at:    CellLevel,
}

impl TerrainPlacementEntry {
        #[must_use]
    pub const fn new(piece: TerrainUuid, at: CellLevel) -> Self {
        Self { piece, at }
    }
}
