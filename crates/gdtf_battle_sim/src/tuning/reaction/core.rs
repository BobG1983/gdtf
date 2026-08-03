use bevy::prelude::{Component, Deref};

use super::leaves::{ReactionTuning, clamp_probability, reaction_cap};
use crate::ganger::{Reactions, Tu, TuMax};


#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct ReactionScore(f32);

impl ReactionScore {
                        #[must_use]
    pub const fn new(score: f32) -> Self {
        Self(score)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct ReactionProbability(f32);

impl ReactionProbability {
                        #[must_use]
    pub const fn new(p: f32) -> Self {
        Self(p)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interrupts(bool);

impl Interrupts {
        #[must_use]
    pub const fn new(interrupts: bool) -> Self {
        Self(interrupts)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MayInterrupt(bool);

impl MayInterrupt {
        #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReactionsUsed(u32);

impl ReactionsUsed {
                        #[must_use]
    pub const fn new(used: u32) -> Self {
        Self(used)
    }

                        pub const fn increment(&mut self) {
        self.0 = self.0.saturating_add(1);
    }

                        pub const fn reset(&mut self) {
        self.0 = 0;
    }
}


#[must_use]
pub fn reaction_score(reactions: Reactions, tu_left: Tu, tu_max: TuMax) -> ReactionScore {
    if *tu_max == 0 {
        return ReactionScore::new(0.0);
    }
    let tu_fraction = f32::from(*tu_left) / f32::from(*tu_max);
    ReactionScore::new(*reactions * tu_fraction)
}

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

#[must_use]
pub fn rolls_interrupt(p: ReactionProbability, rng: &mut crate::rng::ReactionRng) -> Interrupts {
    let roll: f32 = rng.random_range(0.0_f32..1.0);
    Interrupts::new(roll < *p)
}

#[must_use]
pub fn may_interrupt(
    used: ReactionsUsed,
    reactions: Reactions,
    tuning: &ReactionTuning,
) -> MayInterrupt {
    MayInterrupt::new(*used < *reaction_cap(reactions, tuning))
}
