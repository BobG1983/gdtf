//! Coarse terrain kind used by the occupancy grid.

use serde::Deserialize;

use crate::{occupancy::Blocked, terrain::entity::TerrainPieceKind};

/// What kind of terrain occupies a cell for pathing and blocking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum TerrainKind {
    /// Empty floor.
    #[default]
    Open,
    /// Solid wall.
    Wall,
    /// Destructible cover.
    Cover,
    /// Weapon emplacement.
    Emplacement,
}

impl TerrainKind {
    /// Whether this kind blocks movement through the cell.
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
