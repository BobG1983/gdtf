//! Messages and components used by occupancy sync.

use bevy::prelude::{Component, Message};

use crate::{
    metric::{Cell, CellLevel},
    surface::GroundDamage,
};

/// Last occupancy slot for a ganger (and optional stair upper cell).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrevSlot {
    lower: CellLevel,
    upper: Option<CellLevel>,
}

impl PrevSlot {
    /// Single-cell slot.
    #[must_use]
    pub const fn new(lower: CellLevel) -> Self {
        Self { lower, upper: None }
    }

    /// Slot with a stair upper cell.
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

    /// Optional upper stair cell.
    #[must_use]
    pub const fn upper(self) -> Option<CellLevel> {
        self.upper
    }
}

/// Cover at this cell was destroyed.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverDestroyed {
    /// Destroyed cell.
    pub at: CellLevel,
}

impl CoverDestroyed {
    /// Build the message.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self { at }
    }
}

/// Slab at this cell was destroyed.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabDestroyed {
    /// Destroyed cell.
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
    /// Cell that took damage.
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
