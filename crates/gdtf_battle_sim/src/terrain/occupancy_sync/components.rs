use bevy::prelude::{Component, Message};

use crate::{
    metric::{Cell, CellLevel},
    surface::GroundDamage,
};

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrevSlot {
        lower: CellLevel,
                upper: Option<CellLevel>,
}

impl PrevSlot {
                    #[must_use]
    pub const fn new(lower: CellLevel) -> Self {
        Self { lower, upper: None }
    }

                            #[must_use]
    pub const fn with_upper(lower: CellLevel, upper: CellLevel) -> Self {
        Self {
            lower,
            upper: Some(upper),
        }
    }

                #[must_use]
    pub const fn slot(self) -> CellLevel {
        self.lower
    }

                #[must_use]
    pub const fn upper(self) -> Option<CellLevel> {
        self.upper
    }
}

/// is pre-0.18 terminology, identical semantics), so it `#[derive(Message)]` and
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverDestroyed {
            pub at: CellLevel,
}

impl CoverDestroyed {
        #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self { at }
    }
}

/// `Message`/`MessageReader` — `bevy-traps.md` #4), so it `#[derive(Message)]` and is
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabDestroyed {
                pub at: CellLevel,
}

impl SlabDestroyed {
        #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self { at }
    }
}

/// `Message`/`MessageReader` — `bevy-traps.md` #4), so it `#[derive(Message)]` and is
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroundAccrued {
            pub cell:   Cell,
            pub amount: GroundDamage,
}

impl GroundAccrued {
            #[must_use]
    pub const fn new(cell: Cell, amount: GroundDamage) -> Self {
        Self { cell, amount }
    }
}
