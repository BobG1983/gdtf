use crate::{
    aim::{cone_for, stability_for, test::support::*},
    cone::PriorShots,
    cover::CoverLedger,
    faced_cell::faced_cell,
    ganger::{Direction, StanceKind},
    metric::CellLevel,
    stability::{StabilityTerms, SuppressionStability, stability},
    tuning::CombatTuning,
    weapon::Stable,
};

#[test]
fn suppressed_shooter_is_shakier_and_wider() {
    let tuning = CombatTuning::default();
    let wpn = weapon(0.2, 0.1);
    let mode = wpn.fire_mode.single();
    let prior = PriorShots::first();
    let empty = CoverLedger::new();

    let plain_state = ShooterState::new(10, 10, 0, StanceKind::Standing, false, Direction::North);
    let pinned_state = ShooterState::new(10, 10, 0, StanceKind::Standing, false, Direction::North)
        .suppressed_from(9, 10, 0);
    let plain = plain_state.as_shooter();
    let pinned = pinned_state.as_shooter();

    let (plain_mult, plain_recoil) = stability_for(&plain, weapon_terms(&wpn), &empty, &tuning);
    let (pinned_mult, pinned_recoil) = stability_for(&pinned, weapon_terms(&wpn), &empty, &tuning);

    assert!(
        *pinned_mult > *plain_mult,
        "a suppressed shooter must read a HIGHER cone_mult (shakier): pinned {} vs plain {}",
        *pinned_mult,
        *plain_mult,
    );
    assert!(
        *pinned_recoil > *plain_recoil,
        "a suppressed shooter must read a HIGHER recoil_growth (climbs more): pinned {} vs plain {}",
        *pinned_recoil,
        *plain_recoil,
    );

    let plain_theta = cone_for(
        &plain,
        wpn.stats(),
        &mode,
        prior,
        &empty,
        weapon_terms(&wpn),
        &tuning,
    );
    let pinned_theta = cone_for(
        &pinned,
        wpn.stats(),
        &mode,
        prior,
        &empty,
        weapon_terms(&wpn),
        &tuning,
    );
    assert!(
        *pinned_theta > *plain_theta,
        "a suppressed shooter's cone must be strictly WIDER: pinned {} vs plain {}",
        *pinned_theta,
        *plain_theta,
    );
}

#[test]
fn unsuppressed_shooter_is_byte_identical_to_the_no_seam_path() {
    use crate::cover::HeightBand;

    let tuning = CombatTuning::default();
    let state = ShooterState::new(20, 20, 2, StanceKind::Standing, false, Direction::East);
    let shooter = state.as_shooter();
    let stable = Stable::new(false);

    let ledger = ledger_with_faced_cover(&shooter, cover_entry(HeightBand::High));

    let (composer_cone, composer_recoil) = stability_for(
        &shooter,
        StabilityTerms {
            stable,
            ..StabilityTerms::default()
        },
        &ledger,
        &tuning,
    );

    let (cell, level) = faced_cell(shooter.position, shooter.facing);
    let faced = ledger.peek(&CellLevel::new(cell, level));
    let (no_seam_cone, no_seam_recoil) = stability(
        StabilityTerms {
            stable,
            ..StabilityTerms::default()
        },
        *shooter.stance,
        faced,
        SuppressionStability::none(),
        &tuning.cone_stability,
    );

    assert_eq!(
        (*composer_cone).to_bits(),
        (*no_seam_cone).to_bits(),
        "an un-suppressed shooter's cone_mult must be identical to the zero-addend path",
    );
    assert_eq!(
        (*composer_recoil).to_bits(),
        (*no_seam_recoil).to_bits(),
        "an un-suppressed shooter's recoil_growth must be identical to the zero-addend path",
    );
}

#[test]
fn first_round_cone_widening_traces_only_to_cone_mult_not_recoil() {
    let tuning = CombatTuning::default();
    let wpn = weapon(0.2, 0.5);
    let mode = wpn.fire_mode.single();
    let round0 = PriorShots::first();
    let empty = CoverLedger::new();

    let plain_state = ShooterState::new(5, 5, 0, StanceKind::Standing, false, Direction::East);
    let pinned_state = ShooterState::new(5, 5, 0, StanceKind::Standing, false, Direction::East)
        .suppressed_from(4, 5, 0);
    let plain = plain_state.as_shooter();
    let pinned = pinned_state.as_shooter();

    let (plain_mult, _) = stability_for(&plain, weapon_terms(&wpn), &empty, &tuning);
    let (pinned_mult, _) = stability_for(&pinned, weapon_terms(&wpn), &empty, &tuning);

    let plain_theta = cone_for(
        &plain,
        wpn.stats(),
        &mode,
        round0,
        &empty,
        weapon_terms(&wpn),
        &tuning,
    );
    let pinned_theta = cone_for(
        &pinned,
        wpn.stats(),
        &mode,
        round0,
        &empty,
        weapon_terms(&wpn),
        &tuning,
    );

    let plain_residue = *plain_theta / *plain_mult;
    let pinned_residue = *pinned_theta / *pinned_mult;
    assert_eq!(
        plain_residue.to_bits(),
        pinned_residue.to_bits(),
        "on round 0 the recoil term is ×1 for both — θ/cone_mult (the recoil-free residue) \
         must be bit-identical: plain {plain_residue} vs pinned {pinned_residue}",
    );
    assert!(
        *pinned_mult > *plain_mult,
        "the suppressed shooter must have a higher cone_mult for this to be a real test",
    );
}
