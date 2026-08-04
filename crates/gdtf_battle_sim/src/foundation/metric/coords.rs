//! Cubic-voxel coordinates: [`Cell`], [`Level`], [`CellLevel`], [`SimPos`].

use bevy::{
    math::{IVec2, IVec3, Vec3},
    prelude::Deref,
};
use serde::{Deserialize, Serialize};

/// Maximum vertical levels in a battle grid.
pub const MAX_LEVELS: u8 = 8;

/// Integer floor cell (x, y).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(from = "CellDef", into = "CellDef")]
pub struct Cell(IVec2);

impl Cell {
    /// Build a cell from x and y.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self(IVec2::new(x, y))
    }
}

/// Serde shape for [`Cell`].
#[derive(Deserialize, Serialize)]
pub struct CellDef {
    x: i32,
    y: i32,
}

impl From<CellDef> for Cell {
    fn from(def: CellDef) -> Self {
        Self::new(def.x, def.y)
    }
}

impl From<Cell> for CellDef {
    fn from(cell: Cell) -> Self {
        Self {
            x: cell.x,
            y: cell.y,
        }
    }
}

/// Vertical storey index.
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize,
)]
#[serde(transparent)]
pub struct Level(u8);

impl Level {
    /// Wrap a storey index.
    #[must_use]
    pub const fn new(storey: u8) -> Self {
        Self(storey)
    }
}

/// Cell plus level as a single key.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(from = "CellLevelDef", into = "CellLevelDef")]
pub struct CellLevel(IVec3);

impl CellLevel {
    /// Combine cell and level.
    #[must_use]
    pub fn new(cell: Cell, level: Level) -> Self {
        Self(IVec3::new(cell.x, cell.y, i32::from(*level)))
    }

    /// Floor cell only.
    #[must_use]
    pub const fn cell(&self) -> Cell {
        Cell::new(self.0.x, self.0.y)
    }

    /// Storey index.
    #[must_use]
    pub fn level(&self) -> Level {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "z is a storey index constructed from a Level's u8; clamped to \
                      0..=u8::MAX here so the cast cannot wrap or sign-flip"
        )]
        let storey = self.0.z.clamp(0, i32::from(u8::MAX)) as u8;
        Level::new(storey)
    }

    /// Split into cell and level.
    #[must_use]
    pub fn split(&self) -> (Cell, Level) {
        (self.cell(), self.level())
    }
}

/// Serde shape for [`CellLevel`].
#[derive(Deserialize, Serialize)]
pub struct CellLevelDef {
    cell:  Cell,
    level: Level,
}

impl From<CellLevelDef> for CellLevel {
    fn from(def: CellLevelDef) -> Self {
        Self::new(def.cell, def.level)
    }
}

impl From<CellLevel> for CellLevelDef {
    fn from(key: CellLevel) -> Self {
        let (cell, level) = key.split();
        Self { cell, level }
    }
}

/// Continuous world position in sim space.
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct SimPos(Vec3);

impl SimPos {
    /// Build from x, y, z.
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vec3::new(x, y, z))
    }
}

/// World-space center of a cell on a level.
#[must_use]
pub fn cell_center(cell: Cell, level: Level) -> SimPos {
    #[expect(
        clippy::cast_precision_loss,
        reason = "grid coords are tiny (0..60 / 0..8); the f32 conversion is exact for this range"
    )]
    SimPos::new(cell.x as f32 + 0.5, cell.y as f32 + 0.5, f32::from(*level))
}

/// Floor continuous position into cell and level.
#[must_use]
pub fn pos_to_cell(pos: SimPos) -> (Cell, Level) {
    let cell = Cell::new(
        *floor_axis(SimUnit::new(pos.x)),
        *floor_axis(SimUnit::new(pos.y)),
    );
    let storey = (*floor_axis(SimUnit::new(pos.z))).clamp(0, i32::from(u8::MAX));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=u8::MAX above, so this u8 cast cannot truncate or wrap"
    )]
    let level = Level::new(storey as u8);
    (cell, level)
}

fn floor_axis(coord: SimUnit) -> CellUnit {
    let floored = coord.floor();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped to the i32 range below, so the cast cannot wrap; fractional part is gone after floor"
    )]
    let clamped = floored.clamp(i32::MIN as f32, i32::MAX as f32) as i32;
    CellUnit::new(clamped)
}

/// One continuous axis unit.
#[derive(Deref, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SimUnit(f32);

impl SimUnit {
    /// Wrap an f32 value.
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self(value)
    }
}

/// One discrete cell axis unit.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellUnit(i32);

impl CellUnit {
    /// Wrap an i32 value.
    #[must_use]
    pub const fn new(value: i32) -> Self {
        Self(value)
    }
}

/// Distance measured in cells.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellDistance(u32);

impl CellDistance {
    /// Wrap a distance in cells.
    #[must_use]
    pub const fn new(distance: u32) -> Self {
        Self(distance)
    }
}
