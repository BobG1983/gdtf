//! Movement outcome messages.

use bevy::prelude::{Entity, Message};

use crate::metric::Cell;

/// Why a move was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveRejection {
    /// No path to destination.
    Unreachable,
    /// Not enough TU.
    Unaffordable,
    /// Suppression blocked the move.
    Suppressed,
}

/// Move request was rejected.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveRejected {
    /// Actor.
    pub actor:  Entity,
    /// Reason.
    pub reason: MoveRejection,
}

impl MoveRejected {
    /// Build a rejection message.
    #[must_use]
    pub const fn new(actor: Entity, reason: MoveRejection) -> Self {
        Self { actor, reason }
    }
}

/// Actor stepped from one cell to another.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MovementOccurred {
    /// Actor.
    pub actor: Entity,
    /// Previous cell.
    pub from:  Cell,
    /// New cell.
    pub to:    Cell,
}

impl MovementOccurred {
    /// Build a movement message.
    #[must_use]
    pub const fn new(actor: Entity, from: Cell, to: Cell) -> Self {
        Self { actor, from, to }
    }
}
