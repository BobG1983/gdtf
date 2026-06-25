//! The cone-size **result** — the [`ConeAngle`] newtype and [`cone_angle`], which
//! composes `θ_cone` as the product of its five multiplicative factors.

use bevy::prelude::Deref;

use crate::{
    cone::{PriorShots, recoil_factor},
    stability::{ConeMult, RecoilGrowth},
    tuning::AimConeMult,
    weapon::{BaseSpread, Kickback, ModeConeMult},
};

/// The dispersion-cone size **`θ_cone`** — the maximum angular deviation a shot can
/// take (resolution.md §1a: "how wide the spread *can* be"). The product of the
/// five multiplicative cone factors, returned by [`cone_angle`].
///
/// The named angle newtype the cone-size calculation returns (no-bare-types: an
/// angle is a domain value, never a bare `f32`), in the sim's angular unit
/// (radians, matching [`BaseSpread`]) — **zero pixels**. Distinct from the
/// per-factor multipliers ([`ConeMult`] / [`AimConeMult`] / [`ModeConeMult`] /
/// [`crate::cone::RecoilFactor`]), which are dimensionless scales, not an angle.
/// Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct ConeAngle(f32);

impl ConeAngle {
    /// Build a cone angle from its magnitude (radians).
    #[must_use]
    pub const fn new(radians: f32) -> Self {
        Self(radians)
    }
}

/// Compute the §1a cone size **`θ_cone`** as the product of its five multiplicative
/// factors (resolution.md §1a; "What's pure math vs sim" line 147:
/// `cone_angle(base_spread, firemode, prior_shots, kickback, recoil_growth,`
/// `stability, aim) → θ_cone`):
///
/// ```text
/// θ_cone = base_spread × stability × aim × firemode × recoil
/// ```
///
/// - `base_spread` — the weapon's intrinsic angular spread ([`BaseSpread`]).
/// - `firemode` — the per-mode selector term ([`ModeConeMult`], single ≈ 1,
///   full-auto ≥ 1), read by the caller off the weapon's
///   [`crate::weapon::FireMode`] data.
/// - `prior_shots` / `kickback` / `recoil_growth` — fold into the `recoil = 1 +
///   prior_shots × kickback × recoil_growth` factor ([`recoil_factor`]); the first
///   round (zero prior shots) makes recoil the identity ×1, and a steadier
///   `recoil_growth` widens strictly less per prior shot.
/// - `stability` — the E2.2 cone-mult curve output ([`ConeMult`], steadier < 1).
/// - `recoil_growth` — the E2.2 recoil-growth curve output ([`RecoilGrowth`],
///   steadier < 1), damping the recoil widening symmetric with the climb.
/// - `aim` — the Aim-Mode multiplier ([`AimConeMult`]; ×0.6 aimed / 1 hip-fired,
///   from [`crate::cone::aim_cone_mult`]).
///
/// This is the cone WIDTH only — the in-cone sample is the §1b vector (E2.5). All
/// factors are multiplicative, so bracing tightens proportionally (resolution.md
/// §1a). Returns the named [`ConeAngle`] (radians — angular, zero pixels); every
/// factor magnitude comes from weapon / tuning data, none hardcoded.
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
    // θ_cone = base_spread × stability × aim × firemode × recoil — all multiplicative.
    let theta = *base_spread * *stability * *aim * *firemode * *recoil;
    ConeAngle::new(theta)
}
