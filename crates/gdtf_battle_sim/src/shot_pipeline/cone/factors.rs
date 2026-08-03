//! Recoil growth and aim-mode multipliers that feed the cone formula.

use bevy::prelude::Deref;

use crate::{
    ganger::Aiming,
    stability::RecoilGrowth,
    tuning::{AimConeMult, ConeStabilityTuning},
    weapon::Kickback,
};

/// How many shots have already been fired in this volley.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PriorShots(u16);

impl PriorShots {
    /// Build from a raw count.
    #[must_use]
    pub const fn new(prior: u16) -> Self {
        Self(prior)
    }

    /// First shot of a volley.
    #[must_use]
    pub const fn first() -> Self {
        Self(0)
    }
}

/// Multiplier applied to the cone for recoil so far.
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct RecoilFactor(f32);

impl RecoilFactor {
    /// Build from a raw factor.
    #[must_use]
    pub const fn new(factor: f32) -> Self {
        Self(factor)
    }
}

/// Recoil factor from prior shots, kickback, and growth rate.
#[must_use]
pub fn recoil_factor(
    prior_shots: PriorShots,
    kickback: Kickback,
    recoil_growth: RecoilGrowth,
) -> RecoilFactor {
    let prior = f32::from(*prior_shots);
    RecoilFactor::new((prior * *kickback).mul_add(*recoil_growth, 1.0))
}

/// Aim-mode cone multiplier (aimed vs hip-fired).
#[must_use]
pub fn aim_cone_mult(aiming: Aiming, tuning: &ConeStabilityTuning) -> AimConeMult {
    if *aiming {
        tuning.aim_mode.cone_mult
    } else {
        AimConeMult::hip_fired()
    }
}
