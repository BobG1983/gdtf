//! Request to move to a destination cell.

use bevy::prelude::{Entity, Message};

use crate::metric::CellLevel;

/// Move request.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveRequested {
    /// Actor.
    pub actor: Entity,
    /// Destination.
    pub dest: CellLevel,
}

impl MoveRequested {
    /// Build a move request.
    #[must_use]
    pub const fn new(actor: Entity, dest: CellLevel) -> Self {
        Self { actor, dest }
    }
}
