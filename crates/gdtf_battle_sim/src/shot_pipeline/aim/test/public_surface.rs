use crate::{
    aim::{cone_for, stability_for, test::support::*},
    cone::PriorShots,
    cover::{CoverLedger, HeightBand},
    ganger::{Direction, StanceKind},
    stability::StabilityTerms,
    tuning::CombatTuning,
    weapon::Stable,
};

#[test]
fn composers_are_the_public_library_surface_with_zero_pixels() {
    use crate::aim::{Shooter as PubShooter, cone_for as pub_cone_for, stability_for as pub_stab};

    let tuning = CombatTuning::default();
    let state = ShooterState::new(12, 12, 0, StanceKind::Prone, true, Direction::North);
    let stance = state.stance;
    let aiming = state.aiming;
    let position = state.position;
    let facing = state.facing;
    let shooter = PubShooter {
        stance:     &stance,
        aiming:     &aiming,
        position:   &position,
        facing:     &facing,
        suppressed: None,
    };
    let wpn = weapon(0.2, 0.1);
    let mode = wpn.fire_mode.single();
    let ledger = CoverLedger::new();

    let (cone_mult, recoil_growth) = pub_stab(&shooter, weapon_terms(&wpn), &ledger, &tuning);
    let theta = pub_cone_for(
        &shooter,
        wpn.stats(),
        &mode,
        PriorShots::first(),
        &ledger,
        weapon_terms(&wpn),
        &tuning,
    );

    assert!(
        (*cone_mult).is_finite(),
        "cone_mult is dimensionless, finite"
    );
    assert!(
        (*recoil_growth).is_finite(),
        "recoil_growth is dimensionless, finite"
    );
    assert!((*theta).is_finite(), "θ_cone is an angle (radians), finite");
}

#[test]
fn stable_weapon_is_steadier_than_non_stable_facing_empty_equal_under_cover() {
    let tuning = CombatTuning::default();
    let state = ShooterState::new(7, 7, 0, StanceKind::Standing, false, Direction::East);
    let shooter = state.as_shooter();
    let stable_wpn = weapon_tagged(0.2, 0.1, true);
    let plain_wpn = weapon_tagged(0.2, 0.1, false);
    let mode = stable_wpn.fire_mode.single();
    let prior = PriorShots::first();
    let stab = |stable: Stable, cover: &CoverLedger| {
        stability_for(
            &shooter,
            StabilityTerms {
                stable,
                ..StabilityTerms::default()
            },
            cover,
            &tuning,
        )
    };

    let empty = CoverLedger::new();

    let (stable_cone_mult, _) = stab(stable_wpn.stable, &empty);
    let (plain_cone_mult, _) = stab(plain_wpn.stable, &empty);
    assert!(
        *stable_cone_mult < *plain_cone_mult,
        "facing an empty cell, a stable weapon must be strictly steadier (lower \
         cone_mult): stable {} vs plain {}",
        *stable_cone_mult,
        *plain_cone_mult,
    );

    let stable_theta = cone_for(
        &shooter,
        stable_wpn.stats(),
        &mode,
        prior,
        &empty,
        weapon_terms(&stable_wpn),
        &tuning,
    );
    let plain_theta = cone_for(
        &shooter,
        plain_wpn.stats(),
        &mode,
        prior,
        &empty,
        weapon_terms(&plain_wpn),
        &tuning,
    );
    assert!(
        *stable_theta < *plain_theta,
        "facing an empty cell, a stable weapon must have a strictly narrower cone: \
         stable {} vs plain {}",
        *stable_theta,
        *plain_theta,
    );

    let under_cover = ledger_with_faced_cover(&shooter, cover_entry(HeightBand::High));

    let (stable_braced_mult, _) = stab(stable_wpn.stable, &under_cover);
    let (plain_braced_mult, _) = stab(plain_wpn.stable, &under_cover);
    assert_eq!(
        (*stable_braced_mult).to_bits(),
        (*plain_braced_mult).to_bits(),
        "under suitable cover both brace — stable adds nothing, so cone_mult is equal",
    );

    let stable_under = cone_for(
        &shooter,
        stable_wpn.stats(),
        &mode,
        prior,
        &under_cover,
        weapon_terms(&stable_wpn),
        &tuning,
    );
    let plain_under = cone_for(
        &shooter,
        plain_wpn.stats(),
        &mode,
        prior,
        &under_cover,
        weapon_terms(&plain_wpn),
        &tuning,
    );
    assert_eq!(
        (*stable_under).to_bits(),
        (*plain_under).to_bits(),
        "under suitable cover both brace — stable and non-stable cones are equal",
    );
}
