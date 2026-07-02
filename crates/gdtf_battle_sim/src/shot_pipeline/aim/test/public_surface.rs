//! Public-surface composer tests — both composers are reachable through the
//! crate's public re-exports with zero-pixel outputs (AC6), and the `stable` tag
//! makes a weapon steadier facing an empty cell yet equal under suitable cover
//! (AC5 / GTW-199).

use crate::{
    aim::{cone_for, stability_for, test::support::*},
    cone::PriorShots,
    cover::{CoverLedger, HeightBand},
    ganger::{Direction, StanceKind},
    stability::{EmplacementStability, SightStability, TerrainBraced},
    tuning::CombatTuning,
};

/// AC6 — both composers are the public model methods (the HUD-shared callable
/// surface) and carry zero pixels: exercise them through the public crate API
/// as a library caller would, asserting finite angular / dimensionless outputs
/// (a `ConeAngle` is radians, a `ConeMult` dimensionless — never a pixel).
#[test]
fn composers_are_the_public_library_surface_with_zero_pixels() {
    // Reach them via the crate's public re-exports, exactly as the HUD / fire()
    // caller would (proving they are the shared public surface).
    use crate::{Shooter as PubShooter, cone_for as pub_cone_for, stability_for as pub_stab};

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
        // The public-surface probe uses an un-suppressed shooter (GTW-526 identity).
        suppressed: None,
    };
    let wpn = weapon(0.2, 0.1);
    let mode = wpn.fire_mode.single();
    let ledger = CoverLedger::new();

    let (cone_mult, recoil_growth) = pub_stab(
        &shooter,
        wpn.stable,
        TerrainBraced::new(false),
        SightStability::none(),
        EmplacementStability::none(),
        &ledger,
        &tuning,
    );
    let theta = pub_cone_for(
        &shooter,
        wpn.stats(),
        &mode,
        PriorShots::first(),
        &ledger,
        TerrainBraced::new(false),
        EmplacementStability::none(),
        &tuning,
    );

    // Angular / dimensionless outputs, all finite — no pixel anywhere.
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

/// AC5 (GTW-199) — a STABLE weapon yields a strictly steadier `stability_for`
/// (lower `ConeMult`) and a strictly narrower `cone_for` than a non-stable one
/// when BOTH face an EMPTY cell (the stable tag engages the brace
/// unconditionally; the non-stable weapon gets no brace) — and the two are EQUAL
/// when both face cover that suits the stance (the brace already engaged for
/// both). Two weapons differ ONLY in the `stable` tag. Relations only.
#[test]
fn stable_weapon_is_steadier_than_non_stable_facing_empty_equal_under_cover() {
    let tuning = CombatTuning::default();
    // Standing's brace gate is HIGH (resolution.md §1a).
    let state = ShooterState::new(7, 7, 0, StanceKind::Standing, false, Direction::East);
    let shooter = state.as_shooter();
    let stable_wpn = weapon_tagged(0.2, 0.1, true);
    let plain_wpn = weapon_tagged(0.2, 0.1, false);
    let mode = stable_wpn.fire_mode.single();
    let prior = PriorShots::first();
    // The un-scoped / un-terrain-braced stability read this test repeats four times — a local
    // closure keeps the call sites terse (the added GTW-542 `sight` arg is always the identity
    // here, since no fixture weapon is scoped).
    let stab = |stable, cover: &CoverLedger| {
        stability_for(
            &shooter,
            stable,
            TerrainBraced::new(false),
            SightStability::none(),
            EmplacementStability::none(),
            cover,
            &tuning,
        )
    };

    // ---- Facing an EMPTY cell: stable braces, non-stable does not. ----
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
        TerrainBraced::new(false),
        EmplacementStability::none(),
        &tuning,
    );
    let plain_theta = cone_for(
        &shooter,
        plain_wpn.stats(),
        &mode,
        prior,
        &empty,
        TerrainBraced::new(false),
        EmplacementStability::none(),
        &tuning,
    );
    assert!(
        *stable_theta < *plain_theta,
        "facing an empty cell, a stable weapon must have a strictly narrower cone: \
         stable {} vs plain {}",
        *stable_theta,
        *plain_theta,
    );

    // ---- Facing cover that SUITS the stance (HIGH wall): both brace, EQUAL. ----
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
        TerrainBraced::new(false),
        EmplacementStability::none(),
        &tuning,
    );
    let plain_under = cone_for(
        &shooter,
        plain_wpn.stats(),
        &mode,
        prior,
        &under_cover,
        TerrainBraced::new(false),
        EmplacementStability::none(),
        &tuning,
    );
    assert_eq!(
        (*stable_under).to_bits(),
        (*plain_under).to_bits(),
        "under suitable cover both brace — stable and non-stable cones are equal",
    );
}
