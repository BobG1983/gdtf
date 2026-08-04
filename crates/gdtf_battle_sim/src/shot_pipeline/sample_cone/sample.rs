//! Draw one random direction inside the cone.

use bevy::math::Vec3;
use rand::{Rng, RngExt};

use crate::{central_axis::AimDir, cone::ConeAngle, sample_cone::ConcentrationP};

/// Normalized direction of a sampled shot.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct ShotDir(Vec3);

impl ShotDir {
    /// Build from any non-zero direction (normalized).
    #[must_use]
    pub fn from_direction(direction: Vec3) -> Self {
        Self(direction.normalize_or_zero())
    }

    /// Inner direction vector.
    #[must_use]
    pub const fn vec(self) -> Vec3 {
        self.0
    }
}

/// Sample a direction around the aim axis inside the cone.
/// Higher concentration pulls samples closer to the axis.
#[must_use]
pub fn sample_cone_vector(
    aim_dir: AimDir,
    cone: ConeAngle,
    p: ConcentrationP,
    rng: &mut impl Rng,
) -> ShotDir {
    let axis = aim_dir.vec();

    if *cone == 0.0 {
        return ShotDir(axis);
    }

    let radius_draw: f32 = rng.random();
    let theta_shot = *cone * radius_draw.powf(*p);

    let azimuth_draw: f32 = rng.random();
    let phi = azimuth_draw * std::f32::consts::TAU;

    let (u, v) = axis.any_orthonormal_pair();

    let (sin_theta, cos_theta) = theta_shot.sin_cos();
    let (sin_phi, cos_phi) = phi.sin_cos();
    let perpendicular = u * cos_phi + v * sin_phi;
    let dir = axis * cos_theta + perpendicular * sin_theta;
    ShotDir(dir.normalize_or_zero())
}
