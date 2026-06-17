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

mod concentration;
mod sample;

#[cfg(test)]
mod test;

pub use concentration::{ConcentrationP, concentration_p};
pub use sample::{ShotDir, sample_cone_vector};
