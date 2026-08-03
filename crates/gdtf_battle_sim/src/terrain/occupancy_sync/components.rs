//! Messages and markers for occupancy projection.

use bevy::prelude::{Component, Message};

use crate::{
    metric::{Cell, CellLevel},
    surface::GroundDamage,
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

/// Cover at a cell was destroyed.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverDestroyed {
    /// Cell of the cover.
    pub at: CellLevel,
}

impl CoverDestroyed {
    /// Build the message.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self { at }
    }
}

/// Slab at a cell was destroyed.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabDestroyed {
    /// Cell of the slab.
    pub at: CellLevel,
}

impl SlabDestroyed {
    /// Build the message.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self { at }
    }
}

/// Ground damage accrued on a cell.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroundAccrued {
    /// Board cell.
    pub cell: Cell,
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
