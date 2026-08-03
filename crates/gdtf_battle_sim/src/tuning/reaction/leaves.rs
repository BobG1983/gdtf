use bevy::prelude::Deref;
use serde::Deserialize;

use super::{
    core::ReactionProbability,
    suppression::{SuppressionRadius, SuppressionStabilityPenalty},
};
use crate::ganger::Reactions;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReactionCap(u32);

impl ReactionCap {
        #[must_use]
    pub const fn new(cap: u32) -> Self {
        Self(cap)
    }
}


/// `#[serde(transparent)]` lets it parse a bare RON scalar; private inner +
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ReactionCapBase(f32);

impl ReactionCapBase {
                                #[must_use]
    pub const fn new(base: f32) -> Self {
        Self(base)
    }
}

impl Default for ReactionCapBase {
    fn default() -> Self {
        Self(1.0)
    }
}

/// magnitude. `#[serde(transparent)]` lets it parse a bare RON scalar; private
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ReactionCapPerReactions(f32);

impl ReactionCapPerReactions {
                                #[must_use]
    pub const fn new(per_reactions: f32) -> Self {
        Self(per_reactions)
    }
}

impl Default for ReactionCapPerReactions {
    fn default() -> Self {
        Self(0.5)
    }
}

/// value). `#[serde(transparent)]` lets it parse a bare RON scalar; private
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ReactionPMin(f32);

impl ReactionPMin {
                            #[must_use]
    pub const fn new(p_min: f32) -> Self {
        Self(p_min)
    }
}

impl Default for ReactionPMin {
    fn default() -> Self {
        Self(0.05)
    }
}

/// example value). `#[serde(transparent)]` lets it parse a bare RON scalar;
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ReactionPMax(f32);

impl ReactionPMax {
                            #[must_use]
    pub const fn new(p_max: f32) -> Self {
        Self(p_max)
    }
}

impl Default for ReactionPMax {
    fn default() -> Self {
        Self(0.95)
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
pub struct ReactionTuning {
            pub cap_base:            ReactionCapBase,
            pub cap_per_reactions:   ReactionCapPerReactions,
            pub p_min:               ReactionPMin,
            pub p_max:               ReactionPMax,
                pub suppression_radius:  SuppressionRadius,
                        pub suppression_penalty: SuppressionStabilityPenalty,
}


#[derive(Deref, Debug, Clone, Copy, PartialEq)]
struct ReactionCapReal(f32);

impl ReactionCapReal {
        #[must_use]
    const fn new(cap: f32) -> Self {
        Self(cap)
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the caller guards val >= 0.0 and applies floor() before calling here, \
              so the cast is always lossless for any sane tuning magnitude"
)]
const fn floor_to_u32(val: ReactionCapReal) -> ReactionCap {
    ReactionCap::new(val.0 as u32)
}

#[must_use]
pub fn reaction_cap(reactions: Reactions, tuning: &ReactionTuning) -> ReactionCap {
    let raw = (*tuning.cap_per_reactions).mul_add(*reactions, *tuning.cap_base);
    if raw < 0.0 {
        ReactionCap::new(0)
    } else {
        floor_to_u32(ReactionCapReal::new(raw.floor()))
    }
}

#[must_use]
pub fn clamp_probability(p: ReactionProbability, tuning: &ReactionTuning) -> ReactionProbability {
    ReactionProbability::new((*p).clamp(*tuning.p_min, *tuning.p_max))
}
