//! The §1b **in-cone vector** — the [`ShotDir`] newtype and [`sample_cone_vector`],
//! which draws ONE 3D unit direction inside the cone from the injected RNG.

use bevy::math::Vec3;
use rand::{Rng, RngExt};

use crate::{central_axis::AimDir, cone::ConeAngle, sample_cone::ConcentrationP};

/// A sampled **shot direction** — the single 3D unit vector the in-cone draw
/// produces (resolution.md §1b: "ONE 3D unit vector about the central axis"), the
/// shot's true direction the coarse march (§2) flies.
///
/// A named unit-direction newtype over [`Vec3`] (no-bare-types: a shot direction is
/// a domain value, not a bare vector), distinct from the [`AimDir`] central axis it
/// is sampled about: this is one perturbed draw inside the cone, not the axis. Its
/// invariant is **unit length** — always constructed normalised by
/// [`sample_cone_vector`]. Private inner + derived [`Deref`](bevy::prelude::Deref);
/// `z` is the `+z` ("up") component the vertical scatter perturbs.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct ShotDir(Vec3);

impl ShotDir {
    /// The inner unit direction vector, by value.
    ///
    /// A convenience over the derived [`Deref`](bevy::prelude::Deref) for callers
    /// that want the [`Vec3`] itself (e.g. the coarse march) without dereferencing —
    /// the vector is unit length by [`sample_cone_vector`]'s construction.
    #[must_use]
    pub const fn vec(self) -> Vec3 {
        self.0
    }
}

/// Sample the §1b **in-cone shot vector** — ONE 3D unit direction inside the
/// dispersion cone, biased toward dead-center (resolution.md §1b lines 43-53).
///
/// The biased radius and azimuth are each one draw from the injected RNG:
///
/// ```text
/// θ_shot = θ_cone × rand^p   (rand ∈ [0,1); HARD EDGE — θ_shot ≤ θ_cone always)
/// φ      = rand × 2π         (uniform azimuth about the central axis)
/// ```
///
/// then `(θ_shot, φ)` is realised as a single unit vector about `aim_dir`: tilt the
/// axis by `θ_shot` toward an orthonormal in-plane basis selected by `φ`, so the
/// scatter is genuinely **3D** — both a lateral and a vertical deviation appear in
/// the one draw (AC #4), never axis-locked.
///
/// Key properties (the acceptance criteria):
/// * **Unit length** — the result is always a unit vector (AC #1), by normalised
///   construction off the unit `aim_dir` and an orthonormal basis.
/// * **Hard edge** — because `rand ∈ [0,1)` and `p > 0`, `rand^p ∈ [0,1)`, so
///   `θ_shot < θ_cone` strictly and the deviation NEVER exceeds `θ_cone` (AC #2).
/// * **Cone 0 = axis exactly** — a zero `θ_cone` makes `θ_shot = 0`, so the result
///   is `aim_dir` itself, dead-center on the target (AC #3); short-circuited so no
///   f32 drift creeps in.
/// * **Determinism** — both draws come from the injected `&mut impl rand::Rng`
///   ([`crate::rng::SimRng`]'s handle), so the same seed yields the same sample
///   stream (AC #4, AC #6: no global/thread RNG).
///
/// `aim_dir` is the E2.4 [`AimDir`] central axis (a sim-space unit-Vec3 direction —
/// NOT a px point); `p` is the [`crate::sample_cone::concentration_p`] exponent.
/// Angular about a unit-Vec3 — **zero pixels**. Returns the named [`ShotDir`]
/// (no-bare-types).
#[must_use]
pub fn sample_cone_vector(
    aim_dir: AimDir,
    cone: ConeAngle,
    p: ConcentrationP,
    rng: &mut impl Rng,
) -> ShotDir {
    let axis = aim_dir.vec();

    // Cone 0 = the axis EXACTLY (dead-center): no draw can perturb a zero-width
    // cone, so short-circuit to avoid any f32 drift through the trig below (AC #3).
    if *cone == 0.0 {
        return ShotDir(axis);
    }

    // The biased polar radius: θ_shot = θ_cone × rand^p. `rand ∈ [0,1)`, so with
    // p > 0 the power is also in [0,1) and θ_shot < θ_cone — the HARD EDGE (AC #2).
    let radius_draw: f32 = rng.random();
    let theta_shot = *cone * radius_draw.powf(*p);

    // The uniform azimuth about the central axis: φ = rand × 2π (AC #4 — the draw
    // that places the deviation anywhere around the axis, lateral AND vertical).
    let azimuth_draw: f32 = rng.random();
    let phi = azimuth_draw * std::f32::consts::TAU;

    // Build an orthonormal basis (u, v) spanning the plane perpendicular to the
    // unit axis, so (θ_shot, φ) can tilt the axis in ANY direction off it — genuine
    // 3D scatter. `any_orthonormal_pair` returns two unit vectors orthogonal to the
    // axis and to each other; rotating between them by φ sweeps the full cone rim.
    let (u, v) = axis.any_orthonormal_pair();

    // Realise (θ_shot, φ) as one unit vector: stay along the axis by cos(θ_shot),
    // and lean into the perpendicular plane by sin(θ_shot) toward the φ-rotated
    // (u, v) direction. Re-normalise to guarantee the unit-length invariant against
    // f32 drift (AC #1).
    let (sin_theta, cos_theta) = theta_shot.sin_cos();
    let (sin_phi, cos_phi) = phi.sin_cos();
    let perpendicular = u * cos_phi + v * sin_phi;
    let dir = axis * cos_theta + perpendicular * sin_theta;
    ShotDir(dir.normalize_or_zero())
}
