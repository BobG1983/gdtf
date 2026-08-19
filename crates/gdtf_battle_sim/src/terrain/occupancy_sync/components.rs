//! Messages and markers for occupancy projection.

use bevy::prelude::{Component, Message};

use crate::{
    metric::{Cell, CellLevel},
    surface::GroundDamage,
    terrain::entity::TerrainPieceKind,
};

/// Previous occupancy slots for a moving ganger (lower and optional upper).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrevSlot {
    lower: CellLevel,
    upper: Option<CellLevel>,
}

impl PrevSlot {
    /// Single-cell previous slot.
    #[must_use]
    pub const fn new(lower: CellLevel) -> Self {
        Self { lower, upper: None }
    }

    /// Two-cell previous slot (e.g. tall stance).
    #[must_use]
    pub const fn with_upper(lower: CellLevel, upper: CellLevel) -> Self {
        Self {
            lower,
            upper: Some(upper),
        }
    }

    /// Lower cell.
    #[must_use]
    pub const fn slot(self) -> CellLevel {
        self.lower
    }

    /// Optional upper cell.
    #[must_use]
    pub const fn upper(self) -> Option<CellLevel> {
        self.upper
    }
}

/// A terrain piece at a cell was destroyed, and which kind it was.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainPieceDestroyed {
    /// Cell of the piece.
    pub at:   CellLevel,
    /// Kind of piece that was destroyed.
    pub kind: TerrainPieceKind,
}

impl TerrainPieceDestroyed {
    /// Build the message.
    #[must_use]
    pub const fn new(at: CellLevel, kind: TerrainPieceKind) -> Self {
        Self { at, kind }
    }
}

/// Ground damage accrued on a cell.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroundAccrued {
    /// Board cell.
    pub cell:   Cell,
    /// Damage amount.
    pub amount: GroundDamage,
}

impl GroundAccrued {
    /// Build the message.
    #[must_use]
    pub const fn new(cell: Cell, amount: GroundDamage) -> Self {
        Self { cell, amount }
    }
}
