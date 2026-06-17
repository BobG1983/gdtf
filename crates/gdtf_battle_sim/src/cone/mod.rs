//! The §1a **cone-size** calculation — `cone_angle(...) → θ_cone`
//! (`docs/combat/resolution.md` §1a + "What's pure math vs sim" line 147:
//! `cone_angle(base_spread, firemode, prior_shots, kickback, recoil_growth,`
//! `stability, aim) → θ_cone`).
//!
//! Cone size is "how wide the spread *can* be" (resolution.md §1a) — the cone
//! WIDTH only; the in-cone sample (where inside the cone the shot lands) is the
//! §1b vector, a SEPARATE slice (E2.5). [`cone_angle`] computes the maximum
//! angular deviation as the **product of five multiplicative factors**
//! (resolution.md §1a: "All factors are multiplicative"):
//!
//! ```text
//! θ_cone = base_spread × stability × aim × firemode × recoil
//!   base_spread : the weapon's intrinsic spread ([`crate::weapon::BaseSpread`])
//!   stability   : the cone-mult curve output (steadier < 1; [`crate::stability::ConeMult`])
//!   aim         : Aim-Mode ×0.6 aimed · 1 hip-fired ([`crate::tuning::AimConeMult`])
//!   firemode    : the per-mode selector term (single ≈ 1, full-auto ≥ 1; [`crate::weapon::ModeConeMult`])
//!   recoil      : 1 + prior_shots × kickback × recoil_growth   (first shot has 0 prior → ×1)
//! ```
//!
//! `recoil_growth` (the steadier-shooter damper; [`crate::stability::RecoilGrowth`],
//! stability's second curve output) damps the recoil cone-WIDENING the same way it
//! damps the recoil *climb* (resolution.md §1a): a braced/prone shooter not only
//! climbs strictly less but widens strictly less per prior shot. The first round
//! (zero prior shots) is still the identity ×1 regardless of `recoil_growth`.
//!
//! Because the factors multiply, bracing tightens **proportionally**
//! (resolution.md §1a): a steadier `stability` multiplier shrinks a sloppy
//! (large `base_spread`) weapon by MORE absolute angle than a tight one, so
//! setting up the big gun is a real payoff and a sloppy weapon sprays on auto
//! while a tight one stays usable. The factor terms are read from data — the
//! `stability` term is the E2.2 curve output, the `aim` term from the E2.1
//! [`crate::tuning::AimMode`] selected by the ganger's [`crate::ganger::Aiming`]
//! flag, the `firemode` term from the weapon's [`crate::weapon::FireMode`] data,
//! and `kickback` from the weapon — so no cone-factor magnitude is hardcoded
//! (resolution.md §"Coefficients live in the combat-tuning data").
//!
//! Angular / dimensionless — **zero pixels**: every factor is a multiplier on the
//! weapon's angular `base_spread`, and the result is the same angular unit
//! (radians). This layer never touches the cubic-voxel metric.

mod angle;
mod factors;

#[cfg(test)]
mod test;

pub use angle::{ConeAngle, cone_angle};
pub use factors::{PriorShots, RecoilFactor, aim_cone_mult, recoil_factor};
