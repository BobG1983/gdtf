//! Per-terrain move costs and vertical link cost.

use bevy::prelude::Deref;
use serde::Deserialize;

use crate::occupancy::TerrainKind;

/// TU to enter one cell of a terrain kind.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct MoveCost(u8);

impl MoveCost {
    /// Wrap a cost.
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

/// Move costs by terrain kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct MoveCosts {
    /// Open ground.
    pub open:  MoveCost,
    /// Cover tile.
    pub cover: MoveCost,
    /// Wall / emplacement (often impassable for standing).
    pub wall:  MoveCost,
}

impl MoveCosts {
    /// Cost for a terrain kind.
    #[must_use]
    pub const fn cost(&self, terrain: TerrainKind) -> MoveCost {
        match terrain {
            TerrainKind::Open => self.open,
            TerrainKind::Cover => self.cover,
            TerrainKind::Wall | TerrainKind::Emplacement => self.wall,
        }
    }
}

impl Default for MoveCosts {
    fn default() -> Self {
        Self {
            open:  MoveCost(4),
            cover: MoveCost(6),
            wall:  MoveCost(8),
        }
    }
}

/// TU to use a vertical link (ladder / hatch).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct LinkTu(u8);

impl LinkTu {
    /// Wrap a cost.
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for LinkTu {
    fn default() -> Self {
        Self(4)
    }
}
