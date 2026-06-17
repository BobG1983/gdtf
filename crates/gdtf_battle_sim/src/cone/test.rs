//! Tests for the §1a cone-size calculation: `cone_angle` is the product of its five
//! multiplicative factors, the first-shot recoil identity, monotone widening,
//! stability damping, the aim/firemode narrowing, proportional bracing, and the
//! newtype `Deref` house style.

use crate::{
    cone::{ConeAngle, PriorShots, RecoilFactor, aim_cone_mult, cone_angle, recoil_factor},
    ganger::Aiming,
    stability::{ConeMult, RecoilGrowth},
    tuning::{AimConeMult, ConeStabilityTuning},
    weapon::{BaseSpread, Kickback, ModeConeMult},
};

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
    use crate::weapon::{FireMode, FireModeSpec, ModeKind, ModeShots, ModeTuPercent};

    let tuning = ConeStabilityTuning::default();
    // The aimed mult is exactly the tuning's authored aim cone mult (read from
    // data, not a hardcoded literal).
    let aimed = aim_cone_mult(Aiming::new(true), &tuning);
    assert_eq!((*aimed).to_bits(), (*tuning.aim_mode.cone_mult).to_bits());

    // The firemode term is read off the weapon's FireMode data — build an
    // arbitrary single-mode selector and route its cone_mult through cone_angle.
    let fire_mode = FireMode::new(vec![FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.5),
        ModeShots::new(1),
    )]);
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
