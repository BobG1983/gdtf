use bevy::prelude::Deref;
use serde::Deserialize;

/// `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct StanceContribution(f32);

impl StanceContribution {
                #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }
}

/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BraceContribution(f32);

impl BraceContribution {
                #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }
}

/// inner + derived [`Deref`]; `#[serde(transparent)]`. The magnitude is tunable balance DATA (a
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct EmplacementStabilityBonus(f32);

impl EmplacementStabilityBonus {
                #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }
}

/// derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct StabilityCurveCoord(f32);

impl StabilityCurveCoord {
                #[must_use]
    pub const fn new(coord: f32) -> Self {
        Self(coord)
    }
}

/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct AimConeMult(f32);

impl AimConeMult {
            #[must_use]
    pub const fn new(mult: f32) -> Self {
        Self(mult)
    }

                    #[must_use]
    pub const fn hip_fired() -> Self {
        Self(1.0)
    }
}

/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct AimTuPremium(f32);

impl AimTuPremium {
                #[must_use]
    pub const fn new(premium: f32) -> Self {
        Self(premium)
    }
}

/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct RecoilClimb(f32);

impl RecoilClimb {
            #[must_use]
    pub const fn new(climb: f32) -> Self {
        Self(climb)
    }
}

/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ConcentrationCoeff(f32);

impl ConcentrationCoeff {
                #[must_use]
    pub const fn new(coeff: f32) -> Self {
        Self(coeff)
    }
}

/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct AimHeightFrac(f32);

impl AimHeightFrac {
            #[must_use]
    pub const fn new(frac: f32) -> Self {
        Self(frac)
    }
}

/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MuzzleForwardOffset(f32);

impl MuzzleForwardOffset {
            #[must_use]
    pub const fn new(offset: f32) -> Self {
        Self(offset)
    }
}

/// `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MuzzleHeight(f32);

impl MuzzleHeight {
                #[must_use]
    pub const fn new(height: f32) -> Self {
        Self(height)
    }
}

/// [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct SilhouetteTop(f32);

impl SilhouetteTop {
            #[must_use]
    pub const fn new(top: f32) -> Self {
        Self(top)
    }
}
