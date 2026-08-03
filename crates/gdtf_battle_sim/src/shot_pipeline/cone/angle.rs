use bevy::prelude::Deref;

use crate::{
    cone::{PriorShots, recoil_factor},
    stability::{ConeMult, RecoilGrowth},
    tuning::AimConeMult,
    weapon::{BaseSpread, Kickback, ModeConeMult},
};

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct ConeAngle(f32);

impl ConeAngle {
        #[must_use]
    pub const fn new(radians: f32) -> Self {
        Self(radians)
    }
}

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
