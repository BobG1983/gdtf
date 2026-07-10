//! From-Downed act requests — [`StabilizeDownedRequested`] and [`ExecuteDownedRequested`].

use bevy::prelude::{Entity, Message};

/// A **stabilize-downed** act was requested — `actor` stabilizes the downed `target`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor + target refs.
/// [`dispatch_stabilize_downed`](crate::acts::downed::dispatch_stabilize_downed) assembles the
/// [`Actor`](crate::acts::downed::Actor) / [`DownedTarget`](crate::acts::downed::DownedTarget)
/// bundles from the queried components and calls
/// [`stabilize_downed`](crate::acts::downed::stabilize_downed), whose faction gate
/// ([`can_stabilize`](crate::acts::downed::can_stabilize)) holds end-to-end — only an
/// 8-adjacent alive ALLY removes the target's
/// [`BleedingOut`](crate::effects::bleed::BleedingOut) condition (the target stays Downed).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StabilizeDownedRequested {
    /// The acting (would-be stabilizer) ganger.
    pub actor:  Entity,
    /// The downed target to stabilize.
    pub target: Entity,
}

impl StabilizeDownedRequested {
    /// Build a stabilize-downed request for `actor` over `target`.
    #[must_use]
    pub const fn new(actor: Entity, target: Entity) -> Self {
        Self { actor, target }
    }
}

/// An **execute-downed** act was requested — `actor` executes the downed `target`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor + target refs.
/// [`dispatch_execute_downed`](crate::acts::downed::dispatch_execute_downed) assembles the
/// [`Actor`](crate::acts::downed::Actor) / [`DownedTarget`](crate::acts::downed::DownedTarget)
/// bundles from the queried components and calls
/// [`execute_downed`](crate::acts::downed::execute_downed), whose faction gate
/// ([`can_execute`](crate::acts::downed::can_execute)) holds end-to-end — only an
/// 8-adjacent alive ENEMY transitions the target to
/// [`LifeState::Dead`](crate::ganger::LifeState::Dead).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExecuteDownedRequested {
    /// The acting (would-be executor) ganger.
    pub actor:  Entity,
    /// The downed target to execute.
    pub target: Entity,
}

impl ExecuteDownedRequested {
    /// Build an execute-downed request for `actor` over `target`.
    #[must_use]
    pub const fn new(actor: Entity, target: Entity) -> Self {
        Self { actor, target }
    }
}
