//! Final cone angle computation.

use bevy::prelude::Deref;

use crate::{
    cone::{PriorShots, recoil_factor},
    stability::{ConeMult, RecoilGrowth},
    tuning::AimConeMult,
    weapon::{BaseSpread, Kickback, ModeConeMult},
};

/// Half-angle of the shot cone in radians.
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct ConeAngle(f32);

impl ConeAngle {
    /// Build from a raw radian value.
    #[must_use]
    pub const fn new(radians: f32) -> Self {
        Self(radians)
    }
}

/// Compute the cone angle from base spread, mode, recoil, stability, and aim.
#[must_use]
pub fn cone_angle(
    base_spread: BaseSpread,
    firemode: ModeConeMult,
    prior_shots: PriorShots,
    kickback: Kickback,
    recoil_growth: RecoilGrowth,
    stability: ConeMult,
    aim: AimConeMult,
) -> ConeAngle {
    let recoil = recoil_factor(prior_shots, kickback, recoil_growth);
    let theta = *base_spread * *stability * *aim * *firemode * *recoil;
    ConeAngle::new(theta)
}
