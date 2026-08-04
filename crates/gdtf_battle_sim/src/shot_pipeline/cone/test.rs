use crate::{
    cone::{ConeAngle, PriorShots, RecoilFactor, aim_cone_mult, cone_angle, recoil_factor},
    ganger::Aiming,
    stability::{ConeMult, RecoilGrowth},
    tuning::{AimConeMult, ConeStabilityTuning},
    weapon::{BaseSpread, Kickback, ModeConeMult},
};

fn cone_mult(value: f32) -> ConeMult {
    ConeMult::new(value)
}

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

    let recoil = (2.0_f32 * 0.10).mul_add(0.5, 1.0);
    let expected = 0.20_f32 * 0.7 * 0.6 * 1.5 * recoil;
    assert_eq!((*theta).to_bits(), expected.to_bits());
}

#[test]
fn first_shot_recoil_term_is_identity() {
    let base = BaseSpread::new(0.18);
    let firemode = ModeConeMult::new(1.2);
    let kick = Kickback::new(0.25);
    let stab = cone_mult(0.8);
    let aim = AimConeMult::new(0.6);
    let growth = RecoilGrowth::new(0.7);

    assert_eq!(
        (*recoil_factor(PriorShots::first(), kick, RecoilGrowth::new(0.1))).to_bits(),
        1.0_f32.to_bits(),
    );
    assert_eq!(
        (*recoil_factor(PriorShots::first(), kick, RecoilGrowth::new(1.9))).to_bits(),
        1.0_f32.to_bits(),
    );

    let first = cone_angle(base, firemode, PriorShots::first(), kick, growth, stab, aim);
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

#[test]
fn each_prior_shot_widens_the_cone_monotonically() {
    let base = BaseSpread::new(0.15);
    let firemode = ModeConeMult::new(1.0);
    let kick = Kickback::new(0.12);
    let growth = RecoilGrowth::new(0.8);
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
        if shots > 0 {
            assert!(
                *theta > prev,
                "a positive-kickback weapon must widen strictly per prior shot",
            );
        }
        prev = *theta;
    }
}

#[test]
fn steadier_recoil_growth_widens_strictly_less() {
    let base = BaseSpread::new(0.2);
    let firemode = ModeConeMult::new(1.0);
    let prior = PriorShots::new(3);
    let kick = Kickback::new(0.15);
    let stab = cone_mult(0.8);
    let aim = AimConeMult::new(1.0);

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

#[test]
fn full_auto_is_at_or_wider_than_single() {
    let base = BaseSpread::new(0.2);
    let prior = PriorShots::first();
    let kick = Kickback::new(0.1);
    let growth = RecoilGrowth::new(0.5);
    let stab = cone_mult(0.8);
    let aim = AimConeMult::new(1.0);

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

#[test]
fn proportional_bracing_helps_a_sloppy_weapon_more_in_absolute_angle() {
    let firemode = ModeConeMult::new(1.0);
    let prior = PriorShots::first();
    let kick = Kickback::new(0.1);
    let growth = RecoilGrowth::new(0.5);
    let aim = AimConeMult::new(1.0);

    let sloppy_stab = cone_mult(1.0);
    let steady_stab = cone_mult(0.5);

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

#[test]
fn aim_and_firemode_terms_are_read_from_data() {
    use crate::weapon::{FireMode, FireModeSpec, ModeKind, ModeShots, ModeTuPercent};

    let tuning = ConeStabilityTuning::default();
    let aimed = aim_cone_mult(Aiming::new(true), &tuning);
    assert_eq!((*aimed).to_bits(), (*tuning.aim_mode.cone_mult).to_bits());

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
    assert!((*theta).is_finite());
}

#[test]
fn cone_newtypes_deref_to_inner() {
    assert_eq!((*ConeAngle::new(0.3)).to_bits(), 0.3_f32.to_bits());
    assert_eq!((*RecoilFactor::new(1.2)).to_bits(), 1.2_f32.to_bits());
    assert_eq!(*PriorShots::new(4), 4u16);
    assert_eq!(*PriorShots::first(), 0u16);
    assert_eq!((*AimConeMult::hip_fired()).to_bits(), 1.0_f32.to_bits());
}
