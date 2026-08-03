use serde::Deserialize;

use crate::{occupancy::Blocked, terrain::entity::TerrainPieceKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum TerrainKind {
        #[default]
    Open,
        Wall,
            Cover,
                Emplacement,
}

impl TerrainKind {
                                                        #[must_use]
    pub const fn blocks(self) -> Blocked {
        Blocked::new(matches!(self, Self::Wall | Self::Cover | Self::Emplacement))
    }
}

impl From<TerrainPieceKind> for TerrainKind {
                                                                                fn from(kind: TerrainPieceKind) -> Self {
        match kind {
            TerrainPieceKind::Wall => Self::Wall,
            TerrainPieceKind::Cover => Self::Cover,
            TerrainPieceKind::Emplacement => Self::Emplacement,
            TerrainPieceKind::Slab => Self::Open,
        }
    }
}
