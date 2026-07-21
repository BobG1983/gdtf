//! [`ActProvenance`] — WHY an act happened, the discriminator the presenter paces on and
//! the wire can label a reaction with (GTW-727 C5).

use bevy::prelude::Entity;

/// What CAUSED the act an [`ActEntry`](super::ActEntry) records.
///
/// The discriminator that exists nowhere else in the sim today: an interrupt writes a
/// plain [`FireRequested`](crate::acts::FireRequested)
/// ([`reaction::interrupt`](crate::reaction)) that is indistinguishable from a commanded
/// or AI shot once it reaches [`dispatch_fire`](crate::acts::dispatch_fire), and
/// [`ReactionShotFired`](crate::acts::movement::ReactionShotFired) names the INTERRUPTED
/// walker, not the reactor. [`Reaction`](Self::Reaction) is sourced from the
/// [`InterruptDeclared`](crate::reaction::InterruptDeclared) exposure message written
/// beside the interrupt's existing writes — pure exposure, no RNG draw, no fire-result
/// logic.
///
/// The presenter reads it to give a reaction shot its own longer beat (so two reactors
/// firing on one player step read as two distinct events rather than one volley); a QA
/// wire projection can carry it without touching the sim again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActProvenance {
    /// A player-commanded act — the acting ganger belongs to the player faction.
    Commanded,
    /// An enemy-brain act — the acting ganger belongs to a non-player faction acting on
    /// its own turn ([`enemy_ai_turn`](crate::ai::enemy_ai_turn)).
    AiTurn,
    /// A REACTION-fire interrupt — the actor fired out of turn because `interrupted` acted
    /// in its line of sight (`docs/combat/resolution.md` §8).
    Reaction {
        /// The ganger whose act triggered this interrupt — the mover/shooter that was
        /// interrupted. A Bevy [`Entity`] handle (framework plumbing, the only bare type
        /// the no-bare-types rule permits in a payload).
        interrupted: Entity,
    },
    /// A CLOCK-driven fact with no commanding actor — a turn boundary, a bleed/DOT/field
    /// tick, or any other per-round engine beat.
    Clock,
}

impl ActProvenance {
    /// Whether this act was a reaction-fire interrupt — the presenter's dwell selector.
    #[must_use]
    pub const fn is_reaction(self) -> bool {
        matches!(self, Self::Reaction { .. })
    }
}
