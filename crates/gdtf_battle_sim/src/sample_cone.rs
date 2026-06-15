//! The §1b **in-cone vector sample** — where inside the dispersion cone the shot
//! actually goes (`docs/combat/resolution.md` §1b lines 43-53 + "What's pure math
//! vs sim" line 150).
//!
//! Cone *size* (E2.3 [`crate::cone::cone_angle`]) decides how wide the spread CAN
//! be; this module decides where inside that width the single shot lands. The two
//! levers are **independent** (resolution.md §1b): `θ_cone` sets the maximum
//! angular width, while the concentration exponent `p` sets how tightly the draws
//! cluster near dead-center. A wide cone with high `p` still mostly lands on
//! target; a narrow cone with low `p` is bounded but evenly scattered.
//!
//! ```text
//! θ_shot = θ_cone × rand^p        (rand ∈ [0,1) — the biased radius; HARD EDGE: θ_shot ≤ θ_cone always)
//! φ      = rand × 2π              (uniform azimuth about the central axis)
//! p      = concentration_p(Shooting, weapon.accuracy)   (rises with accuracy)
//! ```
//!
//! [`concentration_p`] is the power-law exponent: `p ≈ 1` (low accuracy) scatters
//! evenly out to the cone edge, high `p` clusters near dead-center. The weapon
//! term ([`crate::weapon::Accuracy`]) can exceed 1.0. Both coefficients come from
//! the E2.1 [`crate::tuning::ConcentrationCoeffs`] — nothing hardcoded.
//!
//! [`sample_cone_vector`] samples `(θ_shot, φ)` as **ONE 3D unit vector** about the
//! [`crate::central_axis::AimDir`] central axis — lateral AND vertical scatter in a
//! single draw pair, tilted off the axis by `θ_shot` and rotated to azimuth `φ`
//! about it. A **zero** cone returns the axis EXACTLY (dead-center). Every draw
//! bottoms out in the injected `&mut impl rand::Rng` (the [`crate::rng::SimRng`]
//! handle), so the stream is seed-deterministic (`docs/testing.md`). Angular /
//! dimensionless about a unit-Vec3 direction — **zero pixels**.

use bevy::math::Vec3;
use rand::{Rng, RngExt};

use crate::{central_axis::AimDir, cone::ConeAngle, tuning::ConcentrationCoeffs, weapon::Accuracy};

/// A ganger's **Shooting** computed combat stat — the skill term of the
/// concentration exponent `p = concentration_p(Shooting, weapon.accuracy)`
/// (`docs/combat/stats.md`: Shooting is "live today", `fn(Aim, Reflexes, Cool)`,
/// and "feeds shot concentration, `p = Shooting × weapon accuracy`").
///
/// A domain stat value (not a bare `f32` — no-bare-types), dimensionless: higher
/// Shooting raises `p`, clustering the in-cone draw toward dead-center. The
/// roster-side derivation from attributes is campaign scope; this slice consumes a
/// Shooting value. Private inner + derived [`Deref`](bevy::prelude::Deref).
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct Shooting(f32);

impl Shooting {
    /// Build a Shooting value from its magnitude (dimensionless; higher = steadier
    /// aim → larger concentration `p`).
    #[must_use]
    pub const fn new(shooting: f32) -> Self {
        Self(shooting)
    }
}

/// The **concentration exponent `p`** of the §1b power-law radius `rand^p`
/// (resolution.md §1b). `p ≈ 1` scatters the in-cone draw evenly out to the cone
/// edge; a larger `p` clusters it near dead-center. It rises with accuracy
/// (`Shooting × weapon.accuracy`).
///
/// The named exponent newtype (no-bare-types: the power-law exponent is a domain
/// value, not a bare `f32`), returned by [`concentration_p`] and fed to
/// [`sample_cone_vector`]. Dimensionless — **zero pixels**. Private inner + derived
/// [`Deref`](bevy::prelude::Deref).
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct ConcentrationP(f32);

impl ConcentrationP {
    /// Build a concentration exponent from its magnitude (dimensionless; the
    /// power-law exponent of the §1b biased radius).
    #[must_use]
    pub const fn new(p: f32) -> Self {
        Self(p)
    }
}

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

/// The §1b concentration exponent `p = base + scale × (Shooting × weapon.accuracy)`
/// (resolution.md §1b: `p = concentration, rising with accuracy = Shooting ×
/// weapon.accuracy`; "What's pure math vs sim" line 150: `p =
/// concentration_p(Shooting, weapon.accuracy)`, concentration toward dead-center
/// rising with it).
///
/// The accuracy product `Shooting × weapon.accuracy` is mapped through the E2.1
/// [`ConcentrationCoeffs`] (a `base` exponent at zero accuracy plus a per-accuracy
/// `scale`) — so the curve is a data edit, not a code change (AC #6: coefficients
/// from tuning, none hardcoded). With a positive `scale`, `p` rises monotonically
/// with accuracy (AC #5): higher Shooting or a more accurate weapon yields a larger
/// `p`, which [`sample_cone_vector`] uses to cluster shots nearer the axis. The
/// weapon term [`Accuracy`] may exceed 1.0 (resolution.md §1b).
///
/// Dimensionless — **zero pixels**. Returns the named [`ConcentrationP`]
/// (no-bare-types).
#[must_use]
pub fn concentration_p(
    shooting: Shooting,
    accuracy: Accuracy,
    coeffs: ConcentrationCoeffs,
) -> ConcentrationP {
    // p = base + scale × (Shooting × accuracy). The accuracy product is the §1b
    // `Shooting × weapon.accuracy`; the base/scale coefficients are tuning data.
    let product = *shooting * *accuracy;
    let p = product.mul_add(*coeffs.scale, *coeffs.base);
    ConcentrationP::new(p)
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
/// NOT a px point); `p` is the [`concentration_p`] exponent. Angular about a
/// unit-Vec3 — **zero pixels**. Returns the named [`ShotDir`] (no-bare-types).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        central_axis::{AimDir, climb_aim_dir},
        cone::PriorShots,
        metric::SimPos,
        rng::{BattleSeed, SimRng},
        stability::RecoilGrowth,
        tuning::{ConcentrationCoeff, ConcentrationCoeffs, RecoilClimb},
    };

    /// A loose f32 tolerance for the angular / normalisation checks (trig and
    /// normalisation are not exactly representable).
    const TOL: f32 = 1.0e-5;

    /// Number of seeded draws each distribution assertion sweeps.
    const SAMPLES: usize = 2_000;

    /// Build an [`AimDir`] pointing from `muzzle` toward `aim` (zero recoil → the
    /// untilted unit muzzle→aim axis), reusing the E2.4 constructor so the test
    /// exercises the real central-axis path.
    fn aim_dir_from(muzzle: SimPos, aim: SimPos) -> AimDir {
        climb_aim_dir(
            muzzle,
            aim,
            PriorShots::first(),
            RecoilClimb::new(0.0),
            RecoilGrowth::new(0.0),
        )
    }

    /// Arbitrary (not shipped) concentration coefficients for the relation tests.
    fn coeffs(base: f32, scale: f32) -> ConcentrationCoeffs {
        ConcentrationCoeffs {
            base:  ConcentrationCoeff::new(base),
            scale: ConcentrationCoeff::new(scale),
        }
    }

    /// The angular deviation (radians) between `dir` and the central `axis` — both
    /// unit vectors, so `acos(dot)` clamped into the valid domain.
    fn deviation(axis: Vec3, dir: Vec3) -> f32 {
        axis.dot(dir).clamp(-1.0, 1.0).acos()
    }

    // --- AC #1: the result is a unit vector across many seeded draws.

    #[test]
    fn sampled_vector_is_unit_length_across_many_seeded_draws() {
        let aim = aim_dir_from(SimPos::new(0.0, 0.0, 1.0), SimPos::new(10.0, 3.0, 1.5));
        let cone = ConeAngle::new(0.30);
        let p = ConcentrationP::new(1.5);
        let mut rng = SimRng::from_seed(BattleSeed::new(0xA11C));
        for _ in 0..SAMPLES {
            let shot = sample_cone_vector(aim, cone, p, rng.rng());
            assert!(
                (shot.vec().length() - 1.0).abs() < TOL,
                "sampled shot must be unit length, got {}",
                shot.vec().length(),
            );
        }
    }

    // --- AC #2: HARD EDGE — the deviation from aim_dir is ALWAYS ≤ θ_cone.

    #[test]
    fn deviation_never_exceeds_the_cone_angle() {
        let axis_vec = aim_dir_from(SimPos::new(1.0, 1.0, 1.0), SimPos::new(9.0, 2.0, 2.0));
        let cone = ConeAngle::new(0.25);
        // A LOW p scatters out near the edge — the worst case for the hard edge.
        let p = ConcentrationP::new(1.0);
        let mut rng = SimRng::from_seed(BattleSeed::new(0xED6E));
        for _ in 0..SAMPLES {
            let shot = sample_cone_vector(axis_vec, cone, p, rng.rng());
            let dev = deviation(axis_vec.vec(), shot.vec());
            assert!(
                dev <= *cone + TOL,
                "deviation {dev} must never exceed θ_cone {}",
                *cone,
            );
        }
    }

    // --- AC #3: θ_cone = 0 returns the central axis EXACTLY (dead-center).

    #[test]
    fn zero_cone_returns_the_axis_exactly() {
        let aim = aim_dir_from(SimPos::new(2.0, 2.0, 1.0), SimPos::new(7.0, 5.0, 1.2));
        let cone = ConeAngle::new(0.0);
        let p = ConcentrationP::new(2.0);
        // Even draining several draws, every result is the axis bit-for-bit — the
        // cone-0 short-circuit consumes no entropy and applies no trig.
        let mut rng = SimRng::from_seed(BattleSeed::new(0xDEAD));
        for _ in 0..16 {
            let shot = sample_cone_vector(aim, cone, p, rng.rng());
            assert_eq!(
                shot.vec(),
                aim.vec(),
                "a zero cone must return the central axis exactly",
            );
        }
    }

    // --- AC #4: same seed → same stream; scatter is genuinely 3D.

    #[test]
    fn same_seed_yields_the_same_sample_stream() {
        let aim = aim_dir_from(SimPos::new(0.0, 0.0, 1.0), SimPos::new(8.0, 1.0, 1.0));
        let cone = ConeAngle::new(0.4);
        let p = ConcentrationP::new(1.2);

        let draw_stream = || {
            let mut rng = SimRng::from_seed(BattleSeed::new(0x5EED));
            (0..SAMPLES)
                .map(|_| sample_cone_vector(aim, cone, p, rng.rng()).vec())
                .collect::<Vec<_>>()
        };
        let a = draw_stream();
        let b = draw_stream();
        assert_eq!(a, b, "two identically-seeded RNGs must give one stream");
    }

    #[test]
    fn scatter_is_genuinely_three_dimensional() {
        // A horizontal central axis (+x) so the central axis carries no vertical
        // component: any vertical deviation must come from the sample, not the axis.
        let aim = aim_dir_from(SimPos::new(0.0, 0.0, 1.0), SimPos::new(10.0, 0.0, 1.0));
        let axis = aim.vec();
        assert!(axis.z.abs() < TOL, "the test axis must be horizontal");
        let cone = ConeAngle::new(0.5);
        let p = ConcentrationP::new(1.0);
        let mut rng = SimRng::from_seed(BattleSeed::new(0x3D3D));

        // Track whether BOTH a lateral (in-plane, off the +x axis on the ground) and
        // a vertical (+z/-z) deviation component appear across the sample set.
        let mut saw_lateral = false;
        let mut saw_vertical = false;
        for _ in 0..SAMPLES {
            let shot = sample_cone_vector(aim, cone, p, rng.rng()).vec();
            // Lateral = a ground-plane deviation off the +x axis (a y component);
            // vertical = a z component. A meaningful magnitude, not f32 noise.
            if shot.y.abs() > 1.0e-3 {
                saw_lateral = true;
            }
            if shot.z.abs() > 1.0e-3 {
                saw_vertical = true;
            }
            if saw_lateral && saw_vertical {
                break;
            }
        }
        assert!(
            saw_lateral && saw_vertical,
            "scatter must be genuinely 3D — both lateral ({saw_lateral}) and vertical ({saw_vertical}) deviation must appear",
        );
    }

    // --- AC #5: concentration_p rises with accuracy; larger p clusters nearer the
    // axis; independence from θ_cone (wide cone + high p still concentrates).

    #[test]
    fn concentration_p_rises_with_accuracy() {
        let c = coeffs(1.0, 2.0);
        // Higher Shooting OR a more accurate weapon → a strictly larger p.
        let low = concentration_p(Shooting::new(1.0), Accuracy::new(0.5), c);
        let high_skill = concentration_p(Shooting::new(2.0), Accuracy::new(0.5), c);
        let high_weapon = concentration_p(Shooting::new(1.0), Accuracy::new(1.5), c);
        assert!(
            *high_skill > *low,
            "more Shooting must raise p ({} vs {})",
            *high_skill,
            *low,
        );
        assert!(
            *high_weapon > *low,
            "more weapon accuracy must raise p ({} vs {})",
            *high_weapon,
            *low,
        );
    }

    /// The mean angular deviation over a seeded batch at exponent `p`, for a fixed
    /// axis/cone — the statistic AC #5 / independence compare.
    fn mean_deviation(aim: AimDir, cone: ConeAngle, p: ConcentrationP, seed: u64) -> f32 {
        let mut rng = SimRng::from_seed(BattleSeed::new(seed));
        let axis = aim.vec();
        let total: f32 = (0..SAMPLES)
            .map(|_| deviation(axis, sample_cone_vector(aim, cone, p, rng.rng()).vec()))
            .sum();
        #[expect(
            clippy::cast_precision_loss,
            reason = "SAMPLES is 2_000 — exactly representable as f32; this mean is a test statistic"
        )]
        let count = SAMPLES as f32;
        total / count
    }

    #[test]
    fn larger_p_clusters_samples_nearer_the_axis() {
        let aim = aim_dir_from(SimPos::new(0.0, 0.0, 1.0), SimPos::new(10.0, 2.0, 1.5));
        let cone = ConeAngle::new(0.5);
        // Same seed, same cone — only p differs. A larger p must yield a strictly
        // smaller MEAN angular deviation (tighter clustering near dead-center).
        let low_p = mean_deviation(aim, cone, ConcentrationP::new(1.0), 0x00C0_FFEE);
        let high_p = mean_deviation(aim, cone, ConcentrationP::new(6.0), 0x00C0_FFEE);
        assert!(
            high_p < low_p,
            "a larger p must cluster nearer the axis (mean dev {high_p} < {low_p})",
        );
    }

    #[test]
    fn high_p_concentrates_near_center_even_in_a_wide_cone() {
        // Independence (AC #5): θ_cone sets the MAX width; p sets the clustering. A
        // WIDE cone with HIGH p still keeps the mean deviation a small fraction of
        // the cone width — the two levers are independent.
        let aim = aim_dir_from(SimPos::new(0.0, 0.0, 1.0), SimPos::new(10.0, 0.0, 1.0));
        let wide = ConeAngle::new(1.0); // a deliberately wide cone (radians)
        let high_p = ConcentrationP::new(8.0);
        let mean = mean_deviation(aim, wide, high_p, 0xBEEF);
        assert!(
            mean < *wide * 0.25,
            "high p must concentrate near center even in a wide cone (mean {mean} ≪ θ_cone {})",
            *wide,
        );
    }

    // --- newtype house style: derived Deref reaches the inner value.

    #[test]
    fn shot_dir_derefs_to_inner() {
        let aim = aim_dir_from(SimPos::new(0.0, 0.0, 0.0), SimPos::new(1.0, 0.0, 0.0));
        let mut rng = SimRng::from_seed(BattleSeed::new(1));
        let shot = sample_cone_vector(
            aim,
            ConeAngle::new(0.1),
            ConcentrationP::new(1.0),
            rng.rng(),
        );
        assert_eq!(*shot, shot.vec());
    }

    #[test]
    fn concentration_p_and_shooting_deref_to_inner() {
        assert!((*ConcentrationP::new(2.5) - 2.5).abs() < TOL);
        assert!((*Shooting::new(3.0) - 3.0).abs() < TOL);
    }
}
