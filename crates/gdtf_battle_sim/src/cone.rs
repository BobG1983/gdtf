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

use bevy::prelude::Deref;

use crate::{
    ganger::Aiming,
    stability::{ConeMult, RecoilGrowth},
    tuning::{AimConeMult, ConeStabilityTuning},
    weapon::{BaseSpread, Kickback, ModeConeMult},
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

/// The dispersion-cone size **`θ_cone`** — the maximum angular deviation a shot can
/// take (resolution.md §1a: "how wide the spread *can* be"). The product of the
/// five multiplicative cone factors, returned by [`cone_angle`].
///
/// The named angle newtype the cone-size calculation returns (no-bare-types: an
/// angle is a domain value, never a bare `f32`), in the sim's angular unit
/// (radians, matching [`BaseSpread`]) — **zero pixels**. Distinct from the
/// per-factor multipliers (`ConeMult` / `AimConeMult` / `ModeConeMult` /
/// `RecoilFactor`), which are dimensionless scales, not an angle. Private inner +
/// derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct ConeAngle(f32);

impl ConeAngle {
    /// Build a cone angle from its magnitude (radians).
    #[must_use]
    pub const fn new(radians: f32) -> Self {
        Self(radians)
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
///   from [`aim_cone_mult`]).
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

#[cfg(test)]
mod tests {
    use super::*;

    /// An arbitrary cone multiplier from the stability curve — NOT a shipped
    /// magnitude (the cone-size math is value-agnostic on the actual numbers).
    fn cone_mult(value: f32) -> ConeMult {
        ConeMult::new(value)
    }

    /// C1 (AC #1) — `cone_angle(...)` returns a `ConeAngle` equal to the PRODUCT of
    /// the five factors `base_spread × stability × aim × firemode × recoil`, via the
    /// public surface. Composes known ARBITRARY factor inputs (never shipped
    /// magnitudes) and asserts the result bit-equals their hand-computed product —
    /// the FORM is fixed and assertable.
    #[test]
    fn cone_angle_is_the_product_of_its_five_factors() {
        let base = BaseSpread::new(0.20);
        let firemode = ModeConeMult::new(1.5);
        let prior = PriorShots::new(2);
        let kick = Kickback::new(0.10);
        let growth = RecoilGrowth::new(0.5);
        let stab = cone_mult(0.7);
        let aim = AimConeMult::new(0.6);

        let theta = cone_angle(base, firemode, prior, kick, growth, stab, aim);

        // recoil = 1 + 2 × 0.10 × 0.5 = 1.1; product = 0.20 × 0.7 × 0.6 × 1.5 × 1.1.
        let recoil = (2.0_f32 * 0.10).mul_add(0.5, 1.0);
        let expected = 0.20_f32 * 0.7 * 0.6 * 1.5 * recoil;
        assert_eq!((*theta).to_bits(), expected.to_bits());
    }

    /// C2 (AC #2) — the first shot (zero prior shots) makes the recoil term the
    /// identity ×1 REGARDLESS of `recoil_growth`: the `prior_shots = 0` cone equals
    /// the same call with the recoil term forced to identity (a zero-kickback weapon,
    /// where recoil is ×1 regardless of prior shots). Asserts the first-shot recoil
    /// factor is exactly 1.0 for a steady AND a shaky `recoil_growth`, and the cones
    /// match.
    #[test]
    fn first_shot_recoil_term_is_identity() {
        let base = BaseSpread::new(0.18);
        let firemode = ModeConeMult::new(1.2);
        let kick = Kickback::new(0.25);
        let stab = cone_mult(0.8);
        let aim = AimConeMult::new(0.6);
        let growth = RecoilGrowth::new(0.7);

        // The first-shot recoil factor is exactly the identity ×1 regardless of
        // `recoil_growth` (zero prior shots zeroes the whole growth-scaled term).
        assert_eq!(
            (*recoil_factor(PriorShots::first(), kick, RecoilGrowth::new(0.1))).to_bits(),
            1.0_f32.to_bits(),
        );
        assert_eq!(
            (*recoil_factor(PriorShots::first(), kick, RecoilGrowth::new(1.9))).to_bits(),
            1.0_f32.to_bits(),
        );

        // First-shot cone (prior = 0) with a positive-kickback weapon …
        let first = cone_angle(base, firemode, PriorShots::first(), kick, growth, stab, aim);
        // … equals the same call with recoil forced to identity (kickback = 0, so
        // recoil = 1 for any prior-shot count).
        let no_recoil = cone_angle(
            base,
            firemode,
            PriorShots::new(5),
            Kickback::new(0.0),
            growth,
            stab,
            aim,
        );
        assert_eq!((*first).to_bits(), (*no_recoil).to_bits());
    }

    /// C3 (AC #3) — each additional prior shot widens the cone monotonically for a
    /// positive-kickback weapon with positive growth (`recoil = 1 + prior_shots ×
    /// kickback × recoil_growth`). Over an increasing prior-shot count, `θ_cone` is
    /// strictly non-decreasing (strictly increasing here, since kickback > 0 and
    /// growth > 0). Relation, not magnitude.
    #[test]
    fn each_prior_shot_widens_the_cone_monotonically() {
        let base = BaseSpread::new(0.15);
        let firemode = ModeConeMult::new(1.0);
        let kick = Kickback::new(0.12); // positive kickback
        let growth = RecoilGrowth::new(0.8); // positive growth → widening not damped to nil
        let stab = cone_mult(0.9);
        let aim = AimConeMult::new(1.0);

        let mut prev = f32::NEG_INFINITY;
        for shots in 0u16..6 {
            let theta = cone_angle(
                base,
                firemode,
                PriorShots::new(shots),
                kick,
                growth,
                stab,
                aim,
            );
            assert!(
                *theta >= prev,
                "θ_cone must be non-decreasing across prior shots: {} after {prev} at {shots} shots",
                *theta,
            );
            // Strictly increasing for positive kickback (after the first step).
            if shots > 0 {
                assert!(
                    *theta > prev,
                    "a positive-kickback weapon must widen strictly per prior shot",
                );
            }
            prev = *theta;
        }
    }

    /// C3b (AC #3a, GTW-175) — stability damps the recoil **widening**: a steadier
    /// `recoil_growth` (smaller coefficient) yields a STRICTLY SMALLER `θ_cone` than a
    /// shakier one (larger coefficient) for the same positive `prior_shots` /
    /// `kickback`, all else equal — symmetric with how it damps the climb. Asserted by
    /// RELATION (steadier < shakier), no pinned magnitude. The first shot is exempt
    /// (its identity ×1 is the C2 test).
    #[test]
    fn steadier_recoil_growth_widens_strictly_less() {
        let base = BaseSpread::new(0.2);
        let firemode = ModeConeMult::new(1.0);
        let prior = PriorShots::new(3); // positive prior shots — recoil term is live
        let kick = Kickback::new(0.15); // positive kickback
        let stab = cone_mult(0.8);
        let aim = AimConeMult::new(1.0);

        // A steadier shooter (smaller growth) vs a shakier one (larger growth).
        let steady = RecoilGrowth::new(0.3);
        let shaky = RecoilGrowth::new(0.9);

        let steady_cone = cone_angle(base, firemode, prior, kick, steady, stab, aim);
        let shaky_cone = cone_angle(base, firemode, prior, kick, shaky, stab, aim);

        assert!(
            *steady_cone < *shaky_cone,
            "a steadier recoil_growth must widen strictly less for the same prior \
             shots/kickback: steady {} vs shaky {}",
            *steady_cone,
            *shaky_cone,
        );
    }

    /// C4 (AC #4) — aimed fire (`Aiming(true)`) yields a strictly NARROWER `θ_cone`
    /// than hip-fire (`Aiming(false)`), all else equal — the ×0.6 narrowing applied
    /// via [`aim_cone_mult`] reading tuning. Asserted by RELATION (aimed < hip),
    /// never the literal 0.6.
    #[test]
    fn aimed_fire_is_strictly_narrower_than_hip_fire() {
        let tuning = ConeStabilityTuning::default();
        let base = BaseSpread::new(0.2);
        let firemode = ModeConeMult::new(1.0);
        let prior = PriorShots::first();
        let kick = Kickback::new(0.1);
        let growth = RecoilGrowth::new(0.5);
        let stab = cone_mult(0.8);

        let aimed_mult = aim_cone_mult(Aiming::new(true), &tuning);
        let hip_mult = aim_cone_mult(Aiming::new(false), &tuning);

        // Hip-fired is the identity ×1; aiming reads the (sub-1) tuning narrowing.
        assert_eq!((*hip_mult).to_bits(), 1.0_f32.to_bits());

        let aimed = cone_angle(base, firemode, prior, kick, growth, stab, aimed_mult);
        let hip = cone_angle(base, firemode, prior, kick, growth, stab, hip_mult);
        assert!(
            *aimed < *hip,
            "aimed fire must be strictly narrower than hip-fire: aimed {} vs hip {}",
            *aimed,
            *hip,
        );
    }

    /// C5a (AC #5, first half) — full-auto yields a `θ_cone` at-or-wider than single
    /// for the same weapon (the firemode selector term ≥ 1). With a single term ≈ 1
    /// and a full-auto term ≥ 1 (arbitrary inputs honoring the doc relation), the
    /// full-auto cone is ≥ the single cone. Asserted by relation.
    #[test]
    fn full_auto_is_at_or_wider_than_single() {
        let base = BaseSpread::new(0.2);
        let prior = PriorShots::first();
        let kick = Kickback::new(0.1);
        let growth = RecoilGrowth::new(0.5);
        let stab = cone_mult(0.8);
        let aim = AimConeMult::new(1.0);

        // single ≈ 1, full-auto ≥ 1 (the doc relation; arbitrary inputs).
        let single_term = ModeConeMult::new(1.0);
        let full_auto_term = ModeConeMult::new(1.6);

        let single = cone_angle(base, single_term, prior, kick, growth, stab, aim);
        let full_auto = cone_angle(base, full_auto_term, prior, kick, growth, stab, aim);
        assert!(
            *full_auto >= *single,
            "full-auto must be at-or-wider than single: full-auto {} vs single {}",
            *full_auto,
            *single,
        );
    }

    /// C5b (AC #5, second half) — **proportional bracing**: a steadier `stability`
    /// input narrows a sloppy (large `base_spread`) weapon by MORE absolute angle
    /// than a tight one (the multiplicative-proportional property). Holding the
    /// steadier/sloppier `ConeMult` pair fixed, the absolute angle DROP for the
    /// large-base weapon strictly exceeds the drop for the small-base weapon.
    #[test]
    fn proportional_bracing_helps_a_sloppy_weapon_more_in_absolute_angle() {
        let firemode = ModeConeMult::new(1.0);
        let prior = PriorShots::first();
        let kick = Kickback::new(0.1);
        let growth = RecoilGrowth::new(0.5);
        let aim = AimConeMult::new(1.0);

        // A sloppy stability (larger mult) vs a steadier one (smaller mult).
        let sloppy_stab = cone_mult(1.0);
        let steady_stab = cone_mult(0.5);

        // A large-base (sloppy) weapon and a small-base (tight) weapon.
        let big = BaseSpread::new(0.40);
        let tight = BaseSpread::new(0.05);

        let theta = |base, stab| *cone_angle(base, firemode, prior, kick, growth, stab, aim);

        let big_drop = theta(big, sloppy_stab) - theta(big, steady_stab);
        let tight_drop = theta(tight, sloppy_stab) - theta(tight, steady_stab);
        assert!(
            big_drop > tight_drop,
            "a steadier stability must narrow a sloppy weapon by MORE absolute angle \
             (big drop {big_drop} vs tight drop {tight_drop})",
        );
    }

    /// C6 (AC #6) — the aim mult and firemode term are read from tuning / fire-mode
    /// data (not literals): [`aim_cone_mult`] reads `tuning.aim_mode.cone_mult` when
    /// aiming, and the firemode term is the [`ModeConeMult`] read off a weapon's
    /// [`crate::weapon::FireMode`]. Asserts the aimed mult equals the tuning value
    /// (mechanism, not a pinned literal) and a `FireMode::single().cone_mult` flows
    /// through `cone_angle`.
    #[test]
    fn aim_and_firemode_terms_are_read_from_data() {
        use crate::weapon::{FireMode, FireModeSpec, ModeName, ModeShots, ModeTuPercent};

        let tuning = ConeStabilityTuning::default();
        // The aimed mult is exactly the tuning's authored aim cone mult (read from
        // data, not a hardcoded literal).
        let aimed = aim_cone_mult(Aiming::new(true), &tuning);
        assert_eq!((*aimed).to_bits(), (*tuning.aim_mode.cone_mult).to_bits());

        // The firemode term is read off the weapon's FireMode data — build an
        // arbitrary single-mode selector and route its cone_mult through cone_angle.
        let fire_mode = FireMode::Single {
            single: FireModeSpec::new(
                ModeName::new("single".to_owned()),
                ModeConeMult::new(1.0),
                ModeTuPercent::new(0.5),
                ModeShots::new(1),
            ),
        };
        let firemode_term = fire_mode.single().cone_mult;
        let theta = cone_angle(
            BaseSpread::new(0.2),
            firemode_term,
            PriorShots::first(),
            Kickback::new(0.1),
            RecoilGrowth::new(0.5),
            cone_mult(0.8),
            aimed,
        );
        // The result is finite (no panic / NaN) and the firemode term flowed through.
        assert!((*theta).is_finite());
    }

    /// The result + count newtypes' derived [`Deref`] reaches their inner value.
    /// Built from arbitrary literals — pins the Deref mechanism + target type, not a
    /// magnitude.
    #[test]
    fn cone_newtypes_deref_to_inner() {
        assert_eq!((*ConeAngle::new(0.3)).to_bits(), 0.3_f32.to_bits());
        assert_eq!((*RecoilFactor::new(1.2)).to_bits(), 1.2_f32.to_bits());
        assert_eq!(*PriorShots::new(4), 4u16);
        assert_eq!(*PriorShots::first(), 0u16);
        assert_eq!((*AimConeMult::hip_fired()).to_bits(), 1.0_f32.to_bits());
    }
}
