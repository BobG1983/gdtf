//! Messages and writers for a successful interrupt.

use bevy::{
    ecs::system::SystemParam,
    prelude::{Entity, Message, MessageWriter},
};

use crate::acts::{FireRequested, movement::ReactionShotFired};

/// A reactor interrupted another actor with opportunity fire.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InterruptDeclared {
    /// Who fired the reaction.
    pub reactor: Entity,
    /// Who was interrupted.
    pub interrupted: Entity,
}

impl InterruptDeclared {
    /// Build an interrupt message.
    #[must_use]
    pub const fn new(reactor: Entity, interrupted: Entity) -> Self {
        Self {
            reactor,
            interrupted,
        }
    }
}

/// Writers used when an interrupt succeeds.
#[derive(SystemParam)]
pub struct InterruptSignals<'w> {
    pub(super) fire: MessageWriter<'w, FireRequested>,
    pub(super) halt: MessageWriter<'w, ReactionShotFired>,
    pub(super) declared: MessageWriter<'w, InterruptDeclared>,
}
