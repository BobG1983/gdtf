//! Markers and kinds for terrain entities.

use bevy::prelude::{Component, Deref};

use crate::{cover::HeightBand, metric::CellLevel};

/// Cell occupied by this terrain piece.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainCell(CellLevel);

impl TerrainCell {
    /// Wrap a cell.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self(at)
    }
}

/// Marker: this piece can be braced against.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerrainBrace;

/// Marker: this piece blocks pathfinding.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlocksPathfinding;

/// Vision is blocked up to this height band.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlocksVision(HeightBand);

impl BlocksVision {
    /// Wrap a band.
    #[must_use]
    pub const fn new(band: HeightBand) -> Self {
        Self(band)
    }
}

/// Kind of authored terrain piece.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainPieceKind {
    /// Solid wall.
    Wall,
    /// Destructible cover.
    Cover,
    /// Walkable slab / floor.
    Slab,
    /// Weapon emplacement.
    Emplacement,
}

impl TerrainPieceKind {
    /// All kinds.
    pub const ALL: [Self; 4] = [Self::Wall, Self::Cover, Self::Slab, Self::Emplacement];
}
