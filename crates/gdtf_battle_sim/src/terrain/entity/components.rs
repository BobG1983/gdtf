//! Live terrain entity components.

use bevy::prelude::{Component, Deref};

use crate::{cover::HeightBand, metric::CellLevel};

/// Cell occupied by this terrain entity.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainCell(CellLevel);

impl TerrainCell {
    /// Wrap a cell.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self(at)
    }
}

/// Marker: this slab braces a stair below.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerrainBrace;

/// Marker: blocks pathfinding while present.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlocksPathfinding;

/// Blocks vision up to a height band.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlocksVision(HeightBand);

impl BlocksVision {
    /// Wrap a height band.
    #[must_use]
    pub const fn new(band: HeightBand) -> Self {
        Self(band)
    }
}

/// Runtime kind of a terrain piece entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainPieceKind {
    /// Full wall.
    Wall,
    /// Partial cover.
    Cover,
    /// Floor slab.
    Slab,
    /// Weapon emplacement.
    Emplacement,
}

impl TerrainPieceKind {
    /// All kinds.
    pub const ALL: [Self; 4] = [Self::Wall, Self::Cover, Self::Slab, Self::Emplacement];
}
