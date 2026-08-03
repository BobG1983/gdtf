//! Reaction cap, probability clamps, and tuning bundle.

use bevy::prelude::Deref;
use serde::Deserialize;

use super::{
    core::ReactionProbability,
    suppression::{SuppressionRadius, SuppressionStabilityPenalty},
};
use crate::ganger::Reactions;

/// Maximum reaction shots available this turn.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReactionCap(u32);

impl ReactionCap {
    /// Wrap a cap.
    #[must_use]
    pub const fn new(cap: u32) -> Self {
        Self(cap)
    }
}

/// Base term in the reaction-cap formula.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ReactionCapBase(f32);

impl ReactionCapBase {
    /// Wrap a base value.
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

/// Cap increase per reactions stat point.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ReactionCapPerReactions(f32);

impl ReactionCapPerReactions {
    /// Wrap a slope.
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

/// Minimum interrupt probability after clamp.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ReactionPMin(f32);

impl ReactionPMin {
    /// Wrap a minimum p.
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

/// Maximum interrupt probability after clamp.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ReactionPMax(f32);

impl ReactionPMax {
    /// Wrap a maximum p.
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

/// Full reaction tuning resource section.
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
pub struct ReactionTuning {
    /// Cap base.
    pub cap_base: ReactionCapBase,
    /// Cap per reactions.
    pub cap_per_reactions: ReactionCapPerReactions,
    /// Probability floor.
    pub p_min: ReactionPMin,
    /// Probability ceiling.
    pub p_max: ReactionPMax,
    /// Suppression radius in cells.
    pub suppression_radius: SuppressionRadius,
    /// Stability penalty while suppressed.
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

/// Floor of `cap_base + reactions * cap_per_reactions`.
#[must_use]
pub fn reaction_cap(reactions: Reactions, tuning: &ReactionTuning) -> ReactionCap {
    let raw = (*tuning.cap_per_reactions).mul_add(*reactions, *tuning.cap_base);
    if raw < 0.0 {
        ReactionCap::new(0)
    } else {
        floor_to_u32(ReactionCapReal::new(raw.floor()))
    }
}

/// Clamp probability into `[p_min, p_max]`.
#[must_use]
pub fn clamp_probability(p: ReactionProbability, tuning: &ReactionTuning) -> ReactionProbability {
    ReactionProbability::new((*p).clamp(*tuning.p_min, *tuning.p_max))
}
