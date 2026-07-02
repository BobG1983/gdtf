//! GTW-526 suppression composer tests — a [`Suppressed`](crate::ganger::Suppressed)
//! shooter's `stability_for` reads a LOWER stability (higher `ConeMult`) and its
//! `cone_for` is WIDER than an otherwise-identical un-suppressed shooter (C4a), an
//! un-suppressed shooter is BYTE-IDENTICAL to a run without the seam (C4b), and the
//! first-round cone difference traces entirely to `cone_mult` — the recoil term is the
//! ×1 identity on round 0 (C4c).
//!
//! Every assertion is a RELATION / bit-equality, never a pinned magnitude: the shipped
//! `suppression_penalty` (default 40.0) is a tunable balance value, so these tests only
//! prove the seam's DIRECTION (suppressed = shakier = wider) and its pure-additive
//! IDENTITY when absent.

use crate::{
    aim::{cone_for, stability_for, test::support::*},
    cone::PriorShots,
    cover::CoverLedger,
    faced_cell::faced_cell,
    ganger::{Direction, StanceKind},
    metric::CellLevel,
    stability::{
        EmplacementStability, SightStability, SuppressionStability, TerrainBraced, stability,
    },
    tuning::CombatTuning,
    weapon::Stable,
};

/// C4a — a SUPPRESSED shooter's `stability_for` yields a LOWER stability (a strictly
/// HIGHER `ConeMult`, since a lower score reads a higher cone-mult off the curve) and its
/// `cone_for` is strictly WIDER than an otherwise-identical un-suppressed shooter. Two
/// shooters differ ONLY in the `Suppressed` component; a standing shooter facing an empty
/// cell (raw score = the stand contribution) so the suppression subtraction is not masked
/// by a clamp at the ceiling. Relation only — never the shipped penalty magnitude.
#[test]
fn suppressed_shooter_is_shakier_and_wider() {
    let tuning = CombatTuning::default();
    let wpn = weapon(0.2, 0.1);
    let mode = wpn.fire_mode.single();
    let prior = PriorShots::first();
    let empty = CoverLedger::new();

    // Identical shooters bar the Suppressed component; the suppressor origin is arbitrary
    // (the stability seam reads only WHETHER the shooter is suppressed, not from where).
    let plain_state = ShooterState::new(10, 10, 0, StanceKind::Standing, false, Direction::North);
    let pinned_state = ShooterState::new(10, 10, 0, StanceKind::Standing, false, Direction::North)
        .suppressed_from(9, 10, 0);
    let plain = plain_state.as_shooter();
    let pinned = pinned_state.as_shooter();

    let (plain_mult, plain_recoil) = stability_for(
        &plain,
        wpn.stable,
        TerrainBraced::new(false),
        SightStability::none(),
        EmplacementStability::none(),
        &empty,
        &tuning,
    );
    let (pinned_mult, pinned_recoil) = stability_for(
        &pinned,
        wpn.stable,
        TerrainBraced::new(false),
        SightStability::none(),
        EmplacementStability::none(),
        &empty,
        &tuning,
    );

    // A suppressed shooter is SHAKIER: a lower stability score reads a HIGHER cone-mult
    // (wider) and (with the default steadier-is-less-climb curve) a higher recoil-growth.
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

    // And the composed dispersion cone is strictly WIDER for the suppressed shooter.
    let plain_theta = cone_for(
        &plain,
        wpn.stats(),
        &mode,
        prior,
        &empty,
        TerrainBraced::new(false),
        EmplacementStability::none(),
        &tuning,
    );
    let pinned_theta = cone_for(
        &pinned,
        wpn.stats(),
        &mode,
        prior,
        &empty,
        TerrainBraced::new(false),
        EmplacementStability::none(),
        &tuning,
    );
    assert!(
        *pinned_theta > *plain_theta,
        "a suppressed shooter's cone must be strictly WIDER: pinned {} vs plain {}",
        *pinned_theta,
        *plain_theta,
    );
}

/// C4b — IDENTITY: an UN-suppressed shooter's `stability_for` output is BYTE-IDENTICAL to
/// a direct [`stability`] call passing [`SuppressionStability::none`] (the pure-additive
/// identity), proving the seam vanishes when absent — the un-suppressed path is unchanged
/// from before GTW-526. Driven over a real faced-cover lookup so every other term is live.
#[test]
fn unsuppressed_shooter_is_byte_identical_to_the_no_seam_path() {
    use crate::cover::HeightBand;

    let tuning = CombatTuning::default();
    let state = ShooterState::new(20, 20, 2, StanceKind::Standing, false, Direction::East);
    let shooter = state.as_shooter();
    let stable = Stable::new(false);

    // A HIGH wall at the faced cell (a live brace lookup) so the identity is not the
    // trivial empty-cell path.
    let ledger = ledger_with_faced_cover(&shooter, cover_entry(HeightBand::High));

    let (composer_cone, composer_recoil) = stability_for(
        &shooter,
        stable,
        TerrainBraced::new(false),
        SightStability::none(),
        EmplacementStability::none(),
        &ledger,
        &tuning,
    );

    // The direct verb with the SAME inputs the composer feeds, but passing the identity
    // suppression term — this is exactly the pre-GTW-526 call shape plus a zero addend.
    let (cell, level) = faced_cell(shooter.position, shooter.facing);
    let faced = ledger.peek(&CellLevel::new(cell, level));
    let (no_seam_cone, no_seam_recoil) = stability(
        stable,
        TerrainBraced::new(false),
        *shooter.stance,
        faced,
        EmplacementStability::none(),
        SuppressionStability::none(),
        // Un-scoped weapon — the identity sight term, matching the composer's zero-addend path.
        SightStability::none(),
        &tuning.cone_stability,
    );

    assert_eq!(
        (*composer_cone).to_bits(),
        (*no_seam_cone).to_bits(),
        "an un-suppressed shooter's cone_mult must be byte-identical to the zero-addend path",
    );
    assert_eq!(
        (*composer_recoil).to_bits(),
        (*no_seam_recoil).to_bits(),
        "an un-suppressed shooter's recoil_growth must be byte-identical to the zero-addend path",
    );
}

/// C4c — FIRST-ROUND recoil identity: on round 0 (`PriorShots::first()`) the recoil term
/// is the ×1 identity (`recoil = 1 + prior_shots × kickback × recoil_growth`, and
/// `prior_shots = 0`), so the ONLY thing that widens a suppressed shooter's round-0 cone
/// is the `cone_mult` (stability) factor — the suppression-lowered `recoil_growth` does
/// NOT enter round 0. Proven by the RATIO: `θ(round0) / cone_mult` is bit-identical for
/// the suppressed and un-suppressed shooters (both equal `base_spread × mode × aim`, the
/// recoil-free residue), so recoil plays no part on the first round.
#[test]
fn first_round_cone_widening_traces_only_to_cone_mult_not_recoil() {
    let tuning = CombatTuning::default();
    // A positive-kickback weapon so recoil WOULD widen the cone from round 1 on — making
    // the round-0 recoil-identity a real (not vacuous) property.
    let wpn = weapon(0.2, 0.5);
    let mode = wpn.fire_mode.single();
    let round0 = PriorShots::first();
    let empty = CoverLedger::new();

    let plain_state = ShooterState::new(5, 5, 0, StanceKind::Standing, false, Direction::East);
    let pinned_state = ShooterState::new(5, 5, 0, StanceKind::Standing, false, Direction::East)
        .suppressed_from(4, 5, 0);
    let plain = plain_state.as_shooter();
    let pinned = pinned_state.as_shooter();

    // The stability read (cone_mult is the only stability factor entering the round-0 cone).
    let (plain_mult, _) = stability_for(
        &plain,
        wpn.stable,
        TerrainBraced::new(false),
        SightStability::none(),
        EmplacementStability::none(),
        &empty,
        &tuning,
    );
    let (pinned_mult, _) = stability_for(
        &pinned,
        wpn.stable,
        TerrainBraced::new(false),
        SightStability::none(),
        EmplacementStability::none(),
        &empty,
        &tuning,
    );

    let plain_theta = cone_for(
        &plain,
        wpn.stats(),
        &mode,
        round0,
        &empty,
        TerrainBraced::new(false),
        EmplacementStability::none(),
        &tuning,
    );
    let pinned_theta = cone_for(
        &pinned,
        wpn.stats(),
        &mode,
        round0,
        &empty,
        TerrainBraced::new(false),
        EmplacementStability::none(),
        &tuning,
    );

    // θ(round0) = base_spread × mode_mult × cone_mult × aim, with the recoil term = ×1
    // (prior_shots = 0). So θ / cone_mult is the SAME recoil-free residue for both shooters
    // — the suppression-lowered recoil_growth does not enter round 0.
    let plain_residue = *plain_theta / *plain_mult;
    let pinned_residue = *pinned_theta / *pinned_mult;
    assert_eq!(
        plain_residue.to_bits(),
        pinned_residue.to_bits(),
        "on round 0 the recoil term is ×1 for both — θ/cone_mult (the recoil-free residue) \
         must be bit-identical: plain {plain_residue} vs pinned {pinned_residue}",
    );
    // Sanity: the suppressed shooter's cone_mult really is higher (else the residue equality
    // would be trivially true for identical cones).
    assert!(
        *pinned_mult > *plain_mult,
        "the suppressed shooter must have a higher cone_mult for this to be a real test",
    );
}
