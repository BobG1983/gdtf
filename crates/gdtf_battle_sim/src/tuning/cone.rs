//! Cone and stability tuning coefficients.

use bevy::prelude::Deref;
use serde::Deserialize;

/// Stance contribution to stability score.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct StanceContribution(f32);

impl StanceContribution {
    /// Wrap a contribution value.
    #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }
}

/// Brace contribution to stability score.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BraceContribution(f32);

impl BraceContribution {
    /// Wrap a contribution value.
    #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }
}

/// Extra stability when firing from an emplacement.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct EmplacementStabilityBonus(f32);

impl EmplacementStabilityBonus {
    /// Wrap a bonus value.
    #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }
}

/// Coordinate on a stability response curve.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct StabilityCurveCoord(f32);

impl StabilityCurveCoord {
    /// Wrap a curve coordinate.
    #[must_use]
    pub const fn new(coord: f32) -> Self {
        Self(coord)
    }
}

/// Multiplier applied to the cone when aiming.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct AimConeMult(f32);

impl AimConeMult {
    /// Wrap a multiplier.
    #[must_use]
    pub const fn new(mult: f32) -> Self {
        Self(mult)
    }

    /// Hip-fire baseline (1.0).
    #[must_use]
    pub const fn hip_fired() -> Self {
        Self(1.0)
    }
}

/// Extra TU cost for aimed fire.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct AimTuPremium(f32);

impl AimTuPremium {
    /// Wrap a premium value.
    #[must_use]
    pub const fn new(premium: f32) -> Self {
        Self(premium)
    }
}

/// Recoil climb per shot in a burst.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct RecoilClimb(f32);

impl RecoilClimb {
    /// Wrap a climb value.
    #[must_use]
    pub const fn new(climb: f32) -> Self {
        Self(climb)
    }
}

/// Concentration coefficient for stability.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ConcentrationCoeff(f32);

impl ConcentrationCoeff {
    /// Wrap a coefficient.
    #[must_use]
    pub const fn new(coeff: f32) -> Self {
        Self(coeff)
    }
}

/// Fraction of silhouette height used as aim height.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct AimHeightFrac(f32);

impl AimHeightFrac {
    /// Wrap a fraction.
    #[must_use]
    pub const fn new(frac: f32) -> Self {
        Self(frac)
    }
}

/// Forward offset of the muzzle from the cell center.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MuzzleForwardOffset(f32);

impl MuzzleForwardOffset {
    /// Wrap an offset.
    #[must_use]
    pub const fn new(offset: f32) -> Self {
        Self(offset)
    }
}

/// Muzzle height for a stance.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MuzzleHeight(f32);

impl MuzzleHeight {
    /// Wrap a height.
    #[must_use]
    pub const fn new(height: f32) -> Self {
        Self(height)
    }
}

/// Top of the silhouette for a stance.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct SilhouetteTop(f32);

impl SilhouetteTop {
    /// Wrap a top height.
    #[must_use]
    pub const fn new(top: f32) -> Self {
        Self(top)
    }
}
