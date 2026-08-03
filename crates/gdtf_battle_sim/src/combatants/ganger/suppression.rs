use bevy::prelude::{Component, Deref};

use crate::metric::CellLevel;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SuppressorCell(CellLevel);

impl SuppressorCell {
                        #[must_use]
    pub const fn new(from: CellLevel) -> Self {
        Self(from)
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Suppressed {
            pub from: SuppressorCell,
}

impl Suppressed {
                        #[must_use]
    pub const fn new(from: SuppressorCell) -> Self {
        Self { from }
    }
}
