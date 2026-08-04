use crate::{
    aim::{cone_for, stability_for, test::support::*},
    cone::{PriorShots, aim_cone_mult, cone_angle},
    cover::{CoverLedger, HeightBand},
    ganger::{Direction, StanceKind},
    tuning::CombatTuning,
};

#[test]
fn cone_for_bit_equals_a_hand_composed_cone_angle() {
    let tuning = CombatTuning::default();
    let state = ShooterState::new(8, 8, 0, StanceKind::Crouching, true, Direction::West);
    let shooter = state.as_shooter();
    let wpn = weapon(0.2, 0.1);
    let mode = wpn.fire_mode.single();
    let prior = PriorShots::new(2);

    let ledger = ledger_with_faced_cover(&shooter, cover_entry(HeightBand::Mid));

    let via_composer = cone_for(
        &shooter,
        wpn.stats(),
        &mode,
        prior,
        &ledger,
        weapon_terms(&wpn),
        &tuning,
    );

    let (cone_mult, recoil_growth) = stability_for(&shooter, weapon_terms(&wpn), &ledger, &tuning);
    let aim = aim_cone_mult(*shooter.aiming, &tuning.cone_stability);
    let hand = cone_angle(
        wpn.base_spread,
        mode.cone_mult,
        prior,
        wpn.kickback,
        recoil_growth,
        cone_mult,
        aim,
    );

    assert_eq!(
        (*via_composer).to_bits(),
        (*hand).to_bits(),
        "cone_for must bit-equal a hand-composed cone_angle over the five §1a factors",
    );
}

#[test]
fn aimed_cone_for_is_strictly_narrower_than_hip_fire() {
    let tuning = CombatTuning::default();
    let wpn = weapon(0.2, 0.1);
    let mode = wpn.fire_mode.single();
    let prior = PriorShots::first();

    let aimed_state = ShooterState::new(10, 10, 0, StanceKind::Standing, true, Direction::North);
    let hip_state = ShooterState::new(10, 10, 0, StanceKind::Standing, false, Direction::North);
    let aimed = aimed_state.as_shooter();
    let hip = hip_state.as_shooter();

    let ledger = CoverLedger::new();

    let aimed_cone = cone_for(
        &aimed,
        wpn.stats(),
        &mode,
        prior,
        &ledger,
        weapon_terms(&wpn),
        &tuning,
    );
    let hip_cone = cone_for(
        &hip,
        wpn.stats(),
        &mode,
        prior,
        &ledger,
        weapon_terms(&wpn),
        &tuning,
    );

    assert!(
        *aimed_cone < *hip_cone,
        "aimed fire must be strictly narrower than hip-fire: aimed {} vs hip {}",
        *aimed_cone,
        *hip_cone,
    );
}

#[test]
fn each_prior_shot_widens_cone_for_monotonically() {
    let tuning = CombatTuning::default();
    let state = ShooterState::new(5, 5, 0, StanceKind::Standing, false, Direction::East);
    let shooter = state.as_shooter();
    let wpn = weapon(0.2, 0.15);
    let mode = wpn.fire_mode.single();
    let ledger = CoverLedger::new();

    let mut prev = f32::NEG_INFINITY;
    for shots in 0u16..6 {
        let theta = cone_for(
            &shooter,
            wpn.stats(),
            &mode,
            PriorShots::new(shots),
            &ledger,
            weapon_terms(&wpn),
            &tuning,
        );
        assert!(
            *theta >= prev,
            "θ_cone must be non-decreasing across prior shots: {} after {prev} at {shots}",
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
