//! Who caused an act (command, AI, reaction, or clock).

use bevy::prelude::Entity;

/// Origin of a logged act.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActProvenance {
    /// Player or external command.
    Commanded,
    /// AI turn decision.
    AiTurn,
    /// Reaction that interrupted another actor.
    Reaction {
        /// Actor who was interrupted.
        interrupted: Entity,
    },
    /// Periodic / clock-driven effect.
    Clock,
}

impl ActProvenance {
    /// True when this is a reaction.
    #[must_use]
    pub const fn is_reaction(self) -> bool {
        matches!(self, Self::Reaction { .. })
    }
}
