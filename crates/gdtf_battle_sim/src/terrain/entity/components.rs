use bevy::prelude::{Component, Deref};

use crate::{cover::HeightBand, metric::CellLevel};

#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainCell(CellLevel);

impl TerrainCell {
        #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self(at)
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerrainBrace;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlocksPathfinding;

#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlocksVision(HeightBand);

impl BlocksVision {
                #[must_use]
    pub const fn new(band: HeightBand) -> Self {
        Self(band)
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainPieceKind {
            Wall,
            Cover,
                    Slab,
                            Emplacement,
}

impl TerrainPieceKind {
                                pub const ALL: [Self; 4] = [Self::Wall, Self::Cover, Self::Slab, Self::Emplacement];
}
