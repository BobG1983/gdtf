//! [`InterruptDeclared`] — the reaction-fire EXPOSURE signal (GTW-727 C5).

use bevy::{
    ecs::system::SystemParam,
    prelude::{Entity, Message, MessageWriter},
};

use crate::acts::{FireRequested, movement::ReactionShotFired};

/// A reaction-fire INTERRUPT was declared — `reactor` is firing out of turn because
/// `interrupted` acted in its line of sight (`docs/combat/resolution.md` §8).
///
/// ## Why this message exists
///
/// A successful interrupt writes a plain
/// [`FireRequested`](crate::acts::FireRequested), which is indistinguishable from a
/// commanded or AI-driven shot by the time
/// [`dispatch_fire`](crate::acts::dispatch_fire) resolves it, and the only other signal on
/// that path — [`ReactionShotFired`](crate::acts::movement::ReactionShotFired) — names the
/// INTERRUPTED walker, not the reactor. So nothing downstream could tell that a shot was a
/// reaction, or say whose act provoked it. This message states both.
///
/// It is PURE EXPOSURE of a decision already made: it is written beside the existing
/// interrupt writes, after every gate has passed and the opposed roll has been taken. It
/// adds no RNG draw, reads no fire result, and changes no combat outcome.
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), mirroring
/// [`ReactionShotFired`](crate::acts::movement::ReactionShotFired). Both fields are Bevy
/// [`Entity`] handles — framework plumbing, the only bare type the no-bare-types rule
/// permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InterruptDeclared {
    /// The reacting ganger — the one taking the out-of-turn shot.
    pub reactor:     Entity,
    /// The ganger whose act provoked the interrupt — the one being interrupted.
    pub interrupted: Entity,
}

impl InterruptDeclared {
    /// Build an interrupt-declared signal: `reactor` is interrupting `interrupted`.
    #[must_use]
    pub const fn new(reactor: Entity, interrupted: Entity) -> Self {
        Self {
            reactor,
            interrupted,
        }
    }
}

/// The three output signals a committed interrupt emits, bundled into ONE
/// [`SystemParam`] so [`reaction_trigger`](super::reaction_trigger) stays under Bevy's
/// 16-param system arity (the `FireSignals` bundling precedent).
///
/// A transparent grouping of named output buffers — not itself a wrapped domain value.
#[derive(SystemParam)]
pub struct InterruptSignals<'w> {
    /// C4: the REAL interrupt shot the reactor takes — `dispatch_fire` resolves it as a
    /// normal shot and spends the reactor's TU.
    pub(super) fire:     MessageWriter<'w, FireRequested>,
    /// C4: the walking-actor halt, so an interrupted mover stops at its current cell.
    pub(super) halt:     MessageWriter<'w, ReactionShotFired>,
    /// GTW-727 C5: the interrupt EXPOSURE signal — which reactor is interrupting whose
    /// act. Pure exposure, written only on a committed interrupt; no gate reads it.
    pub(super) declared: MessageWriter<'w, InterruptDeclared>,
}
