//! Grid cell and level coordinates on the wire.

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// X coordinate of a cell.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CellXNet(i32);

impl CellXNet {
    /// Wrap an x value.
    #[must_use]
    pub const fn new(x: i32) -> Self {
        Self(x)
    }
}

/// Y coordinate of a cell.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CellYNet(i32);

impl CellYNet {
    /// Wrap a y value.
    #[must_use]
    pub const fn new(y: i32) -> Self {
        Self(y)
    }
}

/// Vertical storey / level index.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LevelNet(u8);

impl LevelNet {
    /// Wrap a storey index.
    #[must_use]
    pub const fn new(storey: u8) -> Self {
        Self(storey)
    }
}

/// 2D cell on the wire.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellNet {
    /// X coordinate.
    pub x: CellXNet,
    /// Y coordinate.
    pub y: CellYNet,
}

impl CellNet {
    /// Build a cell from x and y.
    #[must_use]
    pub const fn new(x: CellXNet, y: CellYNet) -> Self {
        Self { x, y }
    }
}

/// Cell plus vertical level.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellLevelNet {
    /// Floor cell.
    pub cell:  CellNet,
    /// Storey index.
    pub level: LevelNet,
}

impl CellLevelNet {
    /// Build a cell-level from parts.
    #[must_use]
    pub const fn new(cell: CellNet, level: LevelNet) -> Self {
        Self { cell, level }
    }
}
