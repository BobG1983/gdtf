//! `Blast` vs `SingleTarget` hit-type semantics + seed reproducibility: the blast
//! damages every occupant in the radius, the single shot only its direct target.

use gdtf_battle_sim::{
    acts::FireRequested,
    ganger::Direction,
    metric::{Cell, Level},
    test_support::SituationBuilder,
    weapon::{BlastRadius, HitType},
};

use super::harness::*;

// === AoE: a Blast weapon damages EVERY occupant in the radius (multiple targets, one shot). ===

#[test]
fn a_blast_shot_damages_every_occupant_in_the_radius() {
    let hit = HitType::Blast {
        radius: BlastRadius::new(1),
    };
    let (mut app, seed) = battle_app(0x5541_0A0A, hit);

    // Shooter at (5,5) facing East; the direct target at (8,5); two more clustered within
    // the radius-1 blast disc of the impact (8,5): a teammate at (8,4) (north) and one at
    // (9,5) (east). All the SAME faction (see `target` doc) — the blast striking them is
    // the faction-blind friendly-fire property.
    let situation = SituationBuilder::new()
        .with_gangers([
            shooter(ground(5, 5), Direction::East),
            target(ground(8, 5)),
            target(ground(8, 4)),
            target(ground(9, 5)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(shooter_e), Some(direct), Some(splash_n), Some(splash_e)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(8, 5)),
        ganger_at(&mut app, ground(8, 4)),
        ganger_at(&mut app, ground(9, 5)),
    ) else {
        unreachable!("setup spawns the shooter + three cluster targets at distinct cells");
    };
    assert_ne!(
        shooter_e, direct,
        "the shooter and the direct target are distinct"
    );

    let (direct0, north0, east0) = (
        vitals(&app, direct),
        vitals(&app, splash_n),
        vitals(&app, splash_e),
    );

    // Fire the blast at the direct target's cell.
    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        fire_mode(hit),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);

    // The DIRECT target took damage.
    assert!(
        took_damage(direct0, vitals(&app, direct)),
        "the direct target is damaged by the blast (before {direct0:?}, after {:?})",
        vitals(&app, direct),
    );

    // The SPLASH occupants (a different cell than the impact) each took damage too — the
    // AoE struck MULTIPLE targets from ONE shot. PIN-DISCRIMINATING: with the splash
    // unwired only the direct target would be hurt.
    assert!(
        took_damage(north0, vitals(&app, splash_n)),
        "the northern splash occupant (8,4) is damaged by the radius-1 blast \
         (before {north0:?}, after {:?})",
        vitals(&app, splash_n),
    );
    assert!(
        took_damage(east0, vitals(&app, splash_e)),
        "the eastern splash occupant (9,5) is damaged by the radius-1 blast — the blast is \
         faction-blind (grenades do not discriminate) (before {east0:?}, after {:?})",
        vitals(&app, splash_e),
    );
    assert!(
        is_fielded(&app, shooter_e),
        "the shooter survives its own shot"
    );
}

// === Identity: a Single weapon damages ONLY the direct target — no splash. ===

#[test]
fn a_single_shot_damages_only_the_direct_target_no_splash() {
    let hit = HitType::Single;
    let (mut app, seed) = battle_app(0x5541_0B0B, hit);

    // The same cluster geometry as the blast test (all one faction), but the weapon is a
    // plain Single shot.
    let situation = SituationBuilder::new()
        .with_gangers([
            shooter(ground(5, 5), Direction::East),
            target(ground(8, 5)),
            target(ground(8, 4)),
            target(ground(9, 5)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(shooter_e), Some(direct), Some(bystander_n), Some(bystander_e)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(8, 5)),
        ganger_at(&mut app, ground(8, 4)),
        ganger_at(&mut app, ground(9, 5)),
    ) else {
        unreachable!("setup spawns the shooter + three cluster targets");
    };
    // Post-settle baselines (the all-one-faction cluster has no setup-time combat, so these
    // are clean; the shot under test is the only thing that can change them).
    let (direct0, north0, east0) = (
        vitals(&app, direct),
        vitals(&app, bystander_n),
        vitals(&app, bystander_e),
    );

    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        fire_mode(hit),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);

    // The direct target was hit (a Single shot still works).
    assert!(
        took_damage(direct0, vitals(&app, direct)),
        "the Single shot damages its direct target (before {direct0:?}, after {:?})",
        vitals(&app, direct),
    );

    // The bystanders in adjacent cells are UNTOUCHED — no splash (the identity property):
    // their FULL vitals are unchanged from the post-settle baseline.
    assert_eq!(
        vitals(&app, bystander_n),
        north0,
        "a Single shot does NOT splash the northern bystander (no AoE — the identity property)",
    );
    assert_eq!(
        vitals(&app, bystander_e),
        east0,
        "a Single shot does NOT splash the eastern bystander (no AoE — the identity property)",
    );
}

// === Determinism: the same seed reproduces an identical multi-target blast outcome. ===

#[test]
fn the_blast_outcome_is_reproducible_under_the_same_seed() {
    let hit = HitType::Blast {
        radius: BlastRadius::new(1),
    };
    // Resolve the SAME blast under the SAME seed twice and read the per-target HP after.
    // A deterministic (sorted-order, single-stream) resolution reproduces byte-for-byte.
    let run = |seed: u64| -> Vec<Option<u16>> {
        let (mut app, seed) = battle_app(seed, hit);
        let situation = SituationBuilder::new()
            .with_gangers([
                shooter(ground(5, 5), Direction::East),
                target(ground(8, 5)),
                target(ground(8, 4)),
                target(ground(9, 5)),
            ])
            .build_with_gangs();
        drive_setup(&mut app, seed, situation);
        let Some(shooter_e) = ganger_at(&mut app, ground(5, 5)) else {
            unreachable!("setup spawns the shooter at (5,5)");
        };
        app.world_mut().write_message(FireRequested::new(
            shooter_e,
            fire_mode(hit),
            Cell::new(8, 5),
            Level::new(0),
        ));
        step(&mut app, 3);
        // The per-target HP after, read in a fixed cell order (deterministic readout).
        [ground(8, 5), ground(8, 4), ground(9, 5)]
            .into_iter()
            .map(|at| ganger_at(&mut app, at).and_then(|e| hp_of(&app, e)))
            .collect()
    };

    let a = run(0x5541_0C0C);
    let b = run(0x5541_0C0C);
    assert_eq!(
        a, b,
        "the same BattleSeed reproduces an identical multi-target blast outcome: {a:?} vs {b:?}",
    );
    // Non-vacuous: at least one target actually lost HP (the blast really landed).
    assert!(
        a.iter().flatten().any(|&hp| hp < 100),
        "precondition: the blast actually damaged the cluster (a real, non-vacuous outcome): {a:?}",
    );
}
