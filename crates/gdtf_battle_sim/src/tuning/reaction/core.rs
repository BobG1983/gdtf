//! Reaction score, interrupt probability, and per-turn usage counter.

use bevy::prelude::{Component, Deref};

use super::leaves::{ReactionTuning, clamp_probability, reaction_cap};
use crate::ganger::{Reactions, Tu, TuMax};

/// Score used in the interrupt contest (`reactions * tu_left/tu_max`).
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct ReactionScore(f32);

impl ReactionScore {
    /// Wrap a score.
    #[must_use]
    pub const fn new(score: f32) -> Self {
        Self(score)
    }
}

/// Probability of a successful interrupt roll.
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct ReactionProbability(f32);

impl ReactionProbability {
    /// Wrap a probability.
    #[must_use]
    pub const fn new(p: f32) -> Self {
        Self(p)
    }
}

/// Whether the roll interrupted movement.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interrupts(bool);

impl Interrupts {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(interrupts: bool) -> Self {
        Self(interrupts)
    }
}

/// Whether the watcher still has reaction capacity.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MayInterrupt(bool);

impl MayInterrupt {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

/// Count of reaction shots used this turn.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReactionsUsed(u32);

impl ReactionsUsed {
    /// Wrap a count.
    #[must_use]
    pub const fn new(used: u32) -> Self {
        Self(used)
    }

    /// Increment by one (saturating).
    pub const fn increment(&mut self) {
        self.0 = self.0.saturating_add(1);
    }

    /// Reset to zero.
    pub const fn reset(&mut self) {
        self.0 = 0;
    }
}

/// `reactions * (tu_left / tu_max)`.
#[must_use]
pub fn reaction_score(reactions: Reactions, tu_left: Tu, tu_max: TuMax) -> ReactionScore {
    if *tu_max == 0 {
        return ReactionScore::new(0.0);
    }
    let tu_fraction = f32::from(*tu_left) / f32::from(*tu_max);
    ReactionScore::new(*reactions * tu_fraction)
}

/// Watcher share of `watcher / (watcher + mover)`, clamped.
#[must_use]
pub fn interrupt_probability(
    watcher: ReactionScore,
    mover: ReactionScore,
    tuning: &ReactionTuning,
) -> ReactionProbability {
    let denominator = *watcher + *mover;
    if denominator <= 0.0 {
        return clamp_probability(ReactionProbability::new(0.5), tuning);
    }
    let raw = *watcher / denominator;
    clamp_probability(ReactionProbability::new(raw), tuning)
}

/// Roll against `p`.
#[must_use]
pub fn rolls_interrupt(p: ReactionProbability, rng: &mut crate::rng::ReactionRng) -> Interrupts {
    let roll: f32 = rng.random_range(0.0_f32..1.0);
    Interrupts::new(roll < *p)
}

/// True while used count is below the reaction cap.
#[must_use]
pub fn may_interrupt(
    used: ReactionsUsed,
    reactions: Reactions,
    tuning: &ReactionTuning,
) -> MayInterrupt {
    MayInterrupt::new(*used < *reaction_cap(reactions, tuning))
}
