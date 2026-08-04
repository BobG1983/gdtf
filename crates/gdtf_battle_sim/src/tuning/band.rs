//! Projectile range band edges (fraction of max range).

use bevy::prelude::Deref;
use serde::Deserialize;

/// Single band boundary in 0..1 range fraction.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BandEdge(f32);

impl BandEdge {
    /// Wrap an edge value.
    #[must_use]
    pub const fn new(edge: f32) -> Self {
        Self(edge)
    }
}

/// Low/mid and mid/high band splits.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ProjectileBandEdges {
    /// Boundary between low and mid.
    pub low_mid:  BandEdge,
    /// Boundary between mid and high.
    pub mid_high: BandEdge,
}

impl Default for ProjectileBandEdges {
    fn default() -> Self {
        Self {
            low_mid:  BandEdge(0.33),
            mid_high: BandEdge(0.67),
        }
    }
}
