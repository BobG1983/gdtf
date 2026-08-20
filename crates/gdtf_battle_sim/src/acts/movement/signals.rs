//! Movement outcome messages.

use bevy::prelude::{Entity, Message};

use crate::metric::{Cell, CellLevel};

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

/// A walk finished: one message per completed move, however many cells it stepped.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveCompleted {
    /// Ganger that walked.
    pub mover: Entity,
    /// Cell and level the walk ended on.
    pub at:    CellLevel,
}

impl MoveCompleted {
    /// Build a completed-move message.
    #[must_use]
    pub const fn new(mover: Entity, at: CellLevel) -> Self {
        Self { mover, at }
    }
}
