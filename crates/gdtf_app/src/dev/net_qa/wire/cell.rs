//! Cell and cell-level coordinates on the wire.

use bevy::prelude::Deref;
use gdtf_battle_sim::prelude::{Cell, CellLevel, Level};
use serde::{Deserialize, Serialize};

/// Cell X coordinate.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CellXNet(i32);

impl CellXNet {
    /// Build from a raw X value.
    #[must_use]
    pub const fn new(x: i32) -> Self {
        Self(x)
    }
}

/// Cell Y coordinate.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CellYNet(i32);

impl CellYNet {
    /// Build from a raw Y value.
    #[must_use]
    pub const fn new(y: i32) -> Self {
        Self(y)
    }
}

/// Storey / level index.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LevelNet(u8);

impl LevelNet {
    /// Build from a storey index.
    #[must_use]
    pub const fn new(storey: u8) -> Self {
        Self(storey)
    }
}

/// 2D cell on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellNet {
    /// X coordinate.
    pub x: CellXNet,
    /// Y coordinate.
    pub y: CellYNet,
}

impl CellNet {
    /// Build from X and Y.
    #[must_use]
    pub const fn new(x: CellXNet, y: CellYNet) -> Self {
        Self { x, y }
    }
}

/// Cell plus storey on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellLevelNet {
    /// Floor cell.
    pub cell:  CellNet,
    /// Storey.
    pub level: LevelNet,
}

impl CellLevelNet {
    /// Build from cell and level.
    #[must_use]
    pub const fn new(cell: CellNet, level: LevelNet) -> Self {
        Self { cell, level }
    }

    /// Mirror a sim cell-level key.
    #[must_use]
    pub fn from_sim(at: CellLevel) -> Self {
        let (cell, level) = at.split();
        Self::new(
            CellNet::new(CellXNet::new(cell.x), CellYNet::new(cell.y)),
            LevelNet::new(*level),
        )
    }

    /// Convert back into a sim cell-level key.
    #[must_use]
    pub fn to_sim(self) -> CellLevel {
        CellLevel::new(
            Cell::new(*self.cell.x, *self.cell.y),
            Level::new(*self.level),
        )
    }
}
