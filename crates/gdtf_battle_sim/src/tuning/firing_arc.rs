//! Full firing-arc width in degrees.

use bevy::prelude::Deref;
use serde::Deserialize;

/// Arc width (degrees); half is used for the cone test.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct FiringArc(f32);

impl FiringArc {
    /// Wrap an arc width.
    #[must_use]
    pub const fn new(degrees: f32) -> Self {
        Self(degrees)
    }
}

impl Default for FiringArc {
    fn default() -> Self {
        Self(120.0)
    }
}
