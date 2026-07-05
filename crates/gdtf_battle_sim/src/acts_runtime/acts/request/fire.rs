//! The fire act's request — [`FireRequested`].

use bevy::prelude::{Entity, Message};

use crate::{
    metric::{Cell, Level},
    weapon::FireModeSpec,
};

/// A **fire** act was requested — fire `mode` at `(target_cell, target_level)` for
/// `shooter`.
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), carrying the
/// [`Entity`] shooter ref plus the act's OWNED payload: a [`FireModeSpec`] (owned by value
/// — a `Message` cannot hold a borrow; `Copy` again, GTW-260) plus the target
/// [`Cell`] / [`Level`]. The type has **no
/// lifetime parameter**; [`dispatch_fire`](crate::acts::fire::dispatch_fire) reconstructs the
/// borrow-based [`FireOrder`](crate::fire::FireOrder) `{ mode: &mode, target_cell,
/// target_level }` from this owned payload. The shooter is a Bevy [`Entity`] handle —
/// framework plumbing, the only bare type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct FireRequested {
    /// The firing entity (armed shooter).
    pub shooter:      Entity,
    /// The selected fire mode's per-mode numbers — OWNED (no borrow), so the message has
    /// no lifetime; the dispatch system borrows it into a [`FireOrder`](crate::fire::FireOrder).
    pub mode:         FireModeSpec,
    /// The target cell the player aimed at (the §2 aim cell's x/y).
    pub target_cell:  Cell,
    /// The target storey the player aimed at (the aim cell's z).
    pub target_level: Level,
}

impl FireRequested {
    /// Build a fire request for `shooter` firing `mode` at `(target_cell, target_level)`.
    #[must_use]
    pub const fn new(
        shooter: Entity,
        mode: FireModeSpec,
        target_cell: Cell,
        target_level: Level,
    ) -> Self {
        Self {
            shooter,
            mode,
            target_cell,
            target_level,
        }
    }
}
