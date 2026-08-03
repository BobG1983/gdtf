//! are authored under `reaction:` in `assets/core_tuning/combat.tuning.ron` and
use bevy::prelude::Deref;
use serde::Deserialize;

/// leaf parses, never this magnitude. `#[serde(transparent)]` lets it parse a bare RON
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct SuppressionRadius(u8);

impl SuppressionRadius {
                            #[must_use]
    pub const fn new(radius: u8) -> Self {
        Self(radius)
    }
}

impl Default for SuppressionRadius {
    fn default() -> Self {
        Self(1)
    }
}

/// `#[serde(transparent)]` lets it parse a bare RON scalar; private inner + derived
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct SuppressionStabilityPenalty(f32);

impl SuppressionStabilityPenalty {
                                    #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }
}

impl Default for SuppressionStabilityPenalty {
    fn default() -> Self {
        Self(40.0)
    }
}
