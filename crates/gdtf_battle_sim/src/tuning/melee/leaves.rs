//! Melee margin, multiplier clamp, and fight variance.

use bevy::prelude::Deref;
use serde::Deserialize;

/// Soft margin on the melee contest curve.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MeleeKMargin(f32);

impl MeleeKMargin {
    /// Wrap a margin.
    #[must_use]
    pub const fn new(k_margin: f32) -> Self {
        Self(k_margin)
    }
}

impl Default for MeleeKMargin {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Minimum damage multiplier clamp.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MeleeMultMin(f32);

impl MeleeMultMin {
    /// Wrap a minimum multiplier.
    #[must_use]
    pub const fn new(mult_min: f32) -> Self {
        Self(mult_min)
    }
}

impl Default for MeleeMultMin {
    fn default() -> Self {
        Self(0.5)
    }
}

/// Maximum damage multiplier clamp.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MeleeMultMax(f32);

impl MeleeMultMax {
    /// Wrap a maximum multiplier.
    #[must_use]
    pub const fn new(mult_max: f32) -> Self {
        Self(mult_max)
    }
}

impl Default for MeleeMultMax {
    fn default() -> Self {
        Self(3.0)
    }
}

/// Variance on the fight roll.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct FightVariance(f32);

impl FightVariance {
    /// Wrap a variance value.
    #[must_use]
    pub const fn new(variance: f32) -> Self {
        Self(variance)
    }
}

impl Default for FightVariance {
    fn default() -> Self {
        Self(0.2)
    }
}

/// Melee tuning bundle.
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
pub struct MeleeTuning {
    /// Contest margin.
    pub k_margin: MeleeKMargin,
    /// Min multiplier.
    pub mult_min: MeleeMultMin,
    /// Max multiplier.
    pub mult_max: MeleeMultMax,
    /// Fight variance.
    pub variance: FightVariance,
}
