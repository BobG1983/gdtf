//! The marker a replaced piece wears, and the message an emptied slab cell writes.

use bevy::prelude::{Component, Message};

use crate::metric::CellLevel;

/// Marker: this piece has been replaced and is waiting to be despawned.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReplacedPiece;

/// A destroyed slab left its cell with no slab standing in it.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabLeftOpen {
    /// Cell the destroyed slab stood on.
    pub at: CellLevel,
}

impl SlabLeftOpen {
    /// Build the message.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self { at }
    }
}
