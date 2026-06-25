//! `cone_for` composer tests — it WRAPS a hand-composed `cone_angle` over the
//! five §1a factors (AC3), aimed fire is strictly narrower than hip-fire (AC4),
//! and each prior shot widens the cone monotonically (AC5).

use crate::{
    aim::{cone_for, stability_for, test::support::*},
    cone::{PriorShots, aim_cone_mult, cone_angle},
    cover::{CoverLedger, HeightBand},
    ganger::{Direction, StanceKind},
    tuning::CombatTuning,
};

/// AC3 — `cone_for` returns a `ConeAngle` bit-equal to a hand-composed
/// [`cone_angle`] over the same five §1a factors (the weapon's `base_spread`,
/// the mode's `cone_mult`, the burst's `prior_shots`, the weapon's `kickback`,
/// the `recoil_growth` and `cone_mult` from `stability_for`, and the aim mult):
/// value-agnostic on the inputs, proving the composition WRAPS.
#[test]
fn cone_for_bit_equals_a_hand_composed_cone_angle() {
    let tuning = CombatTuning::default();
    let state = ShooterState::new(8, 8, 0, StanceKind::Crouching, true, Direction::West);
    let shooter = state.as_shooter();
    let wpn = weapon(0.2, 0.1);
    let mode = wpn.fire_mode.single();
    let prior = PriorShots::new(2);

    // Cover at the faced cell (a MID wall — kneel's gate is MID) so the
    // stability term reflects a real faced lookup, not just an empty path.
    let ledger = ledger_with_faced_cover(&shooter, cover_entry(HeightBand::Mid));

    let via_composer = cone_for(&shooter, wpn.stats(), &mode, prior, &ledger, &tuning);

    // Hand-compose: the same stability_for pair + the same aim term + cone_angle.
    // The weapon's `stable` tag is its only stability contribution.
    let (cone_mult, recoil_growth) = stability_for(&shooter, wpn.stable, &ledger, &tuning);
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

/// AC4 — aimed fire (`Aiming(true)`) yields a strictly NARROWER `cone_for`
/// than hip-fire (`Aiming(false)`), all else equal — the ×0.6 via
/// [`aim_cone_mult`], asserted by RELATION (aimed < hip), never the literal
/// 0.6. Two shooters differ ONLY in the aim flag.
#[test]
fn aimed_cone_for_is_strictly_narrower_than_hip_fire() {
    let tuning = CombatTuning::default();
    let wpn = weapon(0.2, 0.1);
    let mode = wpn.fire_mode.single();
    let prior = PriorShots::first();

    // Same posture / position / facing — only the aim flag differs.
    let aimed_state = ShooterState::new(10, 10, 0, StanceKind::Standing, true, Direction::North);
    let hip_state = ShooterState::new(10, 10, 0, StanceKind::Standing, false, Direction::North);
    let aimed = aimed_state.as_shooter();
    let hip = hip_state.as_shooter();

    // Same (empty) cover for both — the only difference is aim.
    let ledger = CoverLedger::new();

    let aimed_cone = cone_for(&aimed, wpn.stats(), &mode, prior, &ledger, &tuning);
    let hip_cone = cone_for(&hip, wpn.stats(), &mode, prior, &ledger, &tuning);

    assert!(
        *aimed_cone < *hip_cone,
        "aimed fire must be strictly narrower than hip-fire: aimed {} vs hip {}",
        *aimed_cone,
        *hip_cone,
    );
}

/// AC5 — each additional prior shot widens `cone_for` monotonically for a
/// positive-kickback weapon (`recoil = 1 + prior_shots × kickback ×
/// recoil_growth`): sweep `prior_shots` and assert `θ_cone` is strictly
/// non-decreasing (the per-round widening E4.5's burst loop relies on).
#[test]
fn each_prior_shot_widens_cone_for_monotonically() {
    let tuning = CombatTuning::default();
    let state = ShooterState::new(5, 5, 0, StanceKind::Standing, false, Direction::East);
    let shooter = state.as_shooter();
    let wpn = weapon(0.2, 0.15); // positive kickback so recoil widens
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
            &tuning,
        );
        assert!(
            *theta >= prev,
            "θ_cone must be non-decreasing across prior shots: {} after {prev} at {shots}",
            *theta,
        );
        // Strictly increasing for positive kickback after the first round
        // (the standing/un-braced score gives a positive recoil_growth).
        if shots > 0 {
            assert!(
                *theta > prev,
                "a positive-kickback weapon must widen strictly per prior shot",
            );
        }
        prev = *theta;
    }
}
