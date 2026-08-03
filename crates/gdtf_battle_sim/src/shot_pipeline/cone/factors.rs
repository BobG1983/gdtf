use bevy::prelude::Deref;

use crate::{
    ganger::Aiming,
    stability::RecoilGrowth,
    tuning::{AimConeMult, ConeStabilityTuning},
    weapon::Kickback,
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PriorShots(u16);

impl PriorShots {
            #[must_use]
    pub const fn new(prior: u16) -> Self {
        Self(prior)
    }

            #[must_use]
    pub const fn first() -> Self {
        Self(0)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct RecoilFactor(f32);

impl RecoilFactor {
        #[must_use]
    pub const fn new(factor: f32) -> Self {
        Self(factor)
    }
}

#[must_use]
pub fn recoil_factor(
    prior_shots: PriorShots,
    kickback: Kickback,
    recoil_growth: RecoilGrowth,
) -> RecoilFactor {
    let prior = f32::from(*prior_shots);
    RecoilFactor::new((prior * *kickback).mul_add(*recoil_growth, 1.0))
}

#[must_use]
pub fn aim_cone_mult(aiming: Aiming, tuning: &ConeStabilityTuning) -> AimConeMult {
    if *aiming {
        tuning.aim_mode.cone_mult
    } else {
        AimConeMult::hip_fired()
    }
}
