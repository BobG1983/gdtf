use bevy::math::Vec3;

use crate::{cone::PriorShots, metric::SimPos, stability::RecoilGrowth, tuning::RecoilClimb};

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct AimDir(Vec3);

impl AimDir {
                        #[must_use]
    pub const fn vec(self) -> Vec3 {
        self.0
    }
}

#[must_use]
pub fn climb_aim_dir(
    muzzle: SimPos,
    aim_point: SimPos,
    prior_shots: PriorShots,
    recoil_climb: RecoilClimb,
    recoil_growth: RecoilGrowth,
) -> AimDir {
    let raw = *aim_point - *muzzle;
    let base = raw.normalize_or_zero();
    let base = if base == Vec3::ZERO { Vec3::X } else { base };

    let tilt = f32::from(*prior_shots) * *recoil_climb * *recoil_growth;
    if tilt == 0.0 {
        return AimDir(base);
    }

    let up = Vec3::Z;
    let up_perp = up - base * up.dot(base);
    if up_perp.length_squared() <= 0.0 {
        return AimDir(base);
    }
    let up_perp = up_perp.normalize_or_zero();

    let (sin, cos) = tilt.sin_cos();
    let rotated = base * cos + up_perp * sin;
    AimDir(rotated.normalize_or_zero())
}
