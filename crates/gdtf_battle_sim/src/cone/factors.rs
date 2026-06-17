//! The cone's **multiplicative factor** newtypes and the two factor functions:
//! the burst index [`PriorShots`], the [`RecoilFactor`] + [`recoil_factor`], and the
//! aim-mode [`aim_cone_mult`]. The product is composed by [`crate::cone::cone_angle`].

use bevy::prelude::Deref;

use crate::{
    ganger::Aiming,
    stability::RecoilGrowth,
    tuning::{AimConeMult, ConeStabilityTuning},
    weapon::Kickback,
};

/// The number of rounds **already fired** this shot action before the round being
/// sized — the `prior_shots` term of the recoil factor `recoil = 1 + prior_shots ×
/// kickback × recoil_growth` (resolution.md §1a: "each round in a burst adds the
/// weapon's kickback, widening the cone for the *next* round"). The **first** round
/// of an action has zero prior shots (→ recoil ×1); recoil resets at the end of each
/// shot action.
///
/// A domain count (the burst index, not a bare `u16`), distinct from
/// [`crate::weapon::ModeShots`] (a mode's total round count) per no-bare-types
/// rule 3. Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PriorShots(u16);

impl PriorShots {
    /// Build a prior-shots count from its round number (`0` for the first round of
    /// a shot action).
    #[must_use]
    pub const fn new(prior: u16) -> Self {
        Self(prior)
    }

    /// The **first round** of a shot action — zero prior shots, so the recoil term
    /// is the identity ×1 (resolution.md §1a: "first shot has 0 prior → ×1").
    #[must_use]
    pub const fn first() -> Self {
        Self(0)
    }
}

/// The **recoil factor** of `θ_cone` — the `recoil = 1 + prior_shots × kickback ×
/// recoil_growth` term (resolution.md §1a). Each prior round in a burst adds the
/// weapon's [`Kickback`], widening the cone for the next round, but stability's
/// [`RecoilGrowth`] (steadier → less) damps that widening — symmetric with how it
/// damps the recoil climb. The first round (zero prior shots) is the identity ×1
/// regardless of `recoil_growth`.
///
/// One of the five multiplicative cone factors, computed by [`recoil_factor`] from
/// the burst's [`PriorShots`], the weapon's [`Kickback`], and stability's
/// [`RecoilGrowth`]. A dimensionless angular multiplier — no pixel. Private inner +
/// derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct RecoilFactor(f32);

impl RecoilFactor {
    /// Build a recoil factor from its magnitude (a dimensionless angular scale).
    #[must_use]
    pub const fn new(factor: f32) -> Self {
        Self(factor)
    }
}

/// The **recoil factor** for a burst round — `recoil = 1 + prior_shots × kickback ×
/// recoil_growth` (resolution.md §1a). The first round (zero prior shots) yields the
/// identity ×1; each additional prior round adds the weapon's [`Kickback`] scaled by
/// stability's [`RecoilGrowth`], so a positive kickback widens the cone monotonically
/// across a burst while a steadier shooter (smaller `recoil_growth`) widens strictly
/// less per prior shot.
///
/// `prior_shots`, `kickback`, and `recoil_growth` are domain newtypes; the arithmetic
/// is the doc form, with `kickback` read from the weapon and `recoil_growth` the E2.2
/// stability output (no magnitude hardcoded). Returns the named [`RecoilFactor`]
/// (no-bare-types).
#[must_use]
pub fn recoil_factor(
    prior_shots: PriorShots,
    kickback: Kickback,
    recoil_growth: RecoilGrowth,
) -> RecoilFactor {
    // `recoil = 1 + prior_shots × kickback × recoil_growth`; cast the count to the
    // angular f32 domain for the multiply. `f32::from` (u16 → f32) is lossless and
    // cannot wrap. The product is added to 1.0, so zero prior shots → identity ×1
    // regardless of `recoil_growth`.
    let prior = f32::from(*prior_shots);
    RecoilFactor::new((prior * *kickback).mul_add(*recoil_growth, 1.0))
}

/// The **aim** cone factor for a shooter — the Aim-Mode multiplier selected by the
/// ganger's [`Aiming`] flag (resolution.md §1a: "aimed **narrows** (×0.6),
/// hip-fired = 1"). Aiming reads the tuning [`crate::tuning::AimMode::cone_mult`]
/// (the ×0.6 narrowing); hip-firing yields the identity ×1.
///
/// The aim multiplier is read from `tuning` (no literal hardcoded — AC #6); the
/// hip-fired identity is [`AimConeMult::hip_fired`] (the multiplicative `1.0`, not
/// a tunable magnitude). Returns the named [`AimConeMult`] (no-bare-types).
#[must_use]
pub fn aim_cone_mult(aiming: Aiming, tuning: &ConeStabilityTuning) -> AimConeMult {
    if *aiming {
        tuning.aim_mode.cone_mult
    } else {
        AimConeMult::hip_fired()
    }
}
