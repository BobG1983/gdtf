//! Stability input terms and output multipliers.

use bevy::prelude::Deref;

use crate::{
    effects::attachments::WeaponBraceBonus, stability::terrain_brace::TerrainBraced, weapon::Stable,
};

/// Inputs that feed the stability score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StabilityTerms {
    /// Weapon is inherently stable (bipod etc).
    pub stable: Stable,
    /// Shooter is braced against terrain.
    pub terrain_braced: TerrainBraced,
    /// Attachment brace bonus.
    pub brace_bonus: WeaponBraceBonus,
    /// Emplacement contribution.
    pub emplacement: EmplacementStability,
}

impl Default for StabilityTerms {
    fn default() -> Self {
        Self {
            stable: Stable::new(false),
            terrain_braced: TerrainBraced::new(false),
            brace_bonus: WeaponBraceBonus::none(),
            emplacement: EmplacementStability::none(),
        }
    }
}

/// Stability points from an emplacement.
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct EmplacementStability(f32);

impl EmplacementStability {
    /// Build from a raw point value.
    #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }

    /// No emplacement contribution.
    #[must_use]
    pub const fn none() -> Self {
        Self(0.0)
    }
}

/// Stability points from suppression (usually negative).
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct SuppressionStability(f32);

impl SuppressionStability {
    /// Build from a raw point value.
    #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }

    /// No suppression effect.
    #[must_use]
    pub const fn none() -> Self {
        Self(0.0)
    }
}

/// Clamped stability score (0–100).
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct StabilityScore(f32);

impl StabilityScore {
    const MIN: f32 = 0.0;
    pub(super) const MAX: f32 = 100.0;

    /// Clamp a raw score into the valid range.
    #[must_use]
    pub const fn clamped(raw: f32) -> Self {
        Self(raw.clamp(Self::MIN, Self::MAX))
    }
}

/// Multiplier applied to the shot cone from stability.
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct ConeMult(f32);

impl ConeMult {
    /// Build from a raw multiplier.
    #[must_use]
    pub const fn new(mult: f32) -> Self {
        Self(mult)
    }
}

/// Recoil growth coefficient from stability.
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct RecoilGrowth(f32);

impl RecoilGrowth {
    /// Build from a raw coefficient.
    #[must_use]
    pub const fn new(coeff: f32) -> Self {
        Self(coeff)
    }
}

/// Intermediate curve sample output.
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub(super) struct CurveOutput(f32);

impl CurveOutput {
    pub(super) const IDENTITY: Self = Self(1.0);

    pub(super) const fn new(output: f32) -> Self {
        Self(output)
    }
}
