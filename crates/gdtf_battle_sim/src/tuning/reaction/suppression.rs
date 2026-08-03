//! Suppression radius and stability penalty.

use bevy::prelude::Deref;
use serde::Deserialize;

/// Cells around a suppressor that apply the pinned-move gate.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct SuppressionRadius(u8);

impl SuppressionRadius {
    /// Wrap a radius.
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

/// Stability points lost while suppressed.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct SuppressionStabilityPenalty(f32);

impl SuppressionStabilityPenalty {
    /// Wrap a penalty.
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
