//! Suppression marker and suppressor origin cell.

use bevy::prelude::{Component, Deref};

use crate::metric::CellLevel;

/// Cell of the unit that caused suppression.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SuppressorCell(CellLevel);

impl SuppressorCell {
    /// Wrap a cell/level.
    #[must_use]
    pub const fn new(from: CellLevel) -> Self {
        Self(from)
    }
}

/// Ganger is suppressed; `from` is the suppressor's cell.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Suppressed {
    /// Where the suppressing fire came from.
    pub from: SuppressorCell,
}

impl Suppressed {
    /// Build a suppression marker.
    #[must_use]
    pub const fn new(from: SuppressorCell) -> Self {
        Self { from }
    }
}
