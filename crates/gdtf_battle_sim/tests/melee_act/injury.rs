//! GTW-821 — a connecting melee strike draws a named injury from the MELEE weighting tables,
//! bridges it to the existing `InjuryInflicted` message, and lands it on the target's ledger;
//! the draw discipline (graze / fatal / miss take NO `InjuryRng` draw) holds on the real path.
//!
//! Every case drives the REAL runtime path — a buffered `MeleeRequested` through
//! `BattleSimPlugin`'s `Simulate` band (`dispatch_melee` → `resolve_melee_strike` → the shared
//! wound core → `InjuryInflicted` → `apply_injury`) — never a hand-called verb.

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    acts::MeleeRequested,
    ganger::Direction,
    rng::{BattleSeed, InjuryRng},
    test_support::SituationBuilder,
    tuning::CombatTuning,
};

use super::{harness::*, injury_content::*};

/// Drive ONE forced-connect melee strike under `tuning` with the per-context injury content
/// installed, returning the app + the (attacker, target) pair.
///
/// The fixture is the shared forced-connect one: a strong attacker 8-adjacent and in LOS of a
/// ZERO-Fight (defenceless) target, so the §7 opposed roll connects on every run regardless of
/// variance — the severity band is then pinned by `tuning`'s edges, not by luck.
fn drive_forced_strike(tuning: CombatTuning) -> (App, Entity, Entity) {
    let mut app = battle_app_with_tuning(tuning);
    with_melee_log(&mut app);
    with_injury_log(&mut app);
    install_per_context_injury_content(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player attacker and one enemy target");
    };
    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);
    (app, attacker, target)
}

/// Whether the app's `InjuryRng` cursor still matches a fresh stream derived from the same
/// battle seed — i.e. the run took ZERO injury draws.
///
/// The melee act is the only injury-drawing path in this fixture (no fire, no fall, no shove),
/// so an unadvanced cursor is exactly "the strike drew nothing" (the in-crate
/// `assert_no_injury_draw` idiom, lifted to the live app).
fn injury_stream_unadvanced(app: &mut App) -> bool {
    let mut reference = InjuryRng::from_root(BattleSeed::new(SEED));
    let expected = reference.next_u64();
    let mut live = app.world_mut().resource_mut::<InjuryRng>();
    live.next_u64() == expected
}

// === A connecting, non-graze / non-fatal strike rolls a MELEE-table injury, emits the
// existing InjuryInflicted, and lands it on the target's ledger. ===

#[test]
fn connecting_melee_wound_rolls_a_melee_table_injury_onto_the_ledger() {
    let (app, _attacker, target) = drive_forced_strike(wounding_tuning());

    // The strike connected on the real path (the FX signal the act emits).
    assert!(
        melee_hits(&app) >= 1,
        "fixture precondition: the forced strike connects (a MeleeResolved was emitted)",
    );

    // The §8 roll happened and bridged out as the EXISTING InjuryInflicted message, addressed
    // to the struck ganger.
    let emitted = injuries_emitted_for(&app, target);
    assert!(
        !emitted.is_empty(),
        "a non-graze, non-fatal melee wound must emit an InjuryInflicted for the target",
    );

    // …and it came from the MELEE per-source table: the melee buckets are the ONLY ones naming
    // this key (the ranged / fall buckets name the decoy), so sampling the wrong context FAILS
    // here instead of passing on "some injury rolled".
    assert!(
        emitted.iter().all(|name| name == MELEE_ONLY),
        "the melee wound must roll from the MELEE weighting table (expected only \
         {MELEE_ONLY:?}, got {emitted:?} — {OTHER_ONLY:?} means the wrong context was sampled)",
    );

    // …and the `apply_injury` boundary folded it onto the target's durable ledger, so it
    // carries into the battle report / campaign exactly as a ranged one does.
    let ledger = ledger_names(&app, target);
    assert!(
        ledger.iter().any(|name| name == MELEE_ONLY),
        "the rolled melee injury must land on the target's InflictedInjuries ledger, got \
         {ledger:?}",
    );
}

// === Determinism: the same seed twice yields the same injury outcome. ===

#[test]
fn melee_injury_is_deterministic_under_the_same_seed() {
    let run = || -> Vec<String> {
        let (app, _attacker, target) = drive_forced_strike(wounding_tuning());
        ledger_names(&app, target)
    };
    let first = run();
    assert!(
        !first.is_empty(),
        "fixture precondition: the seeded strike rolls an injury",
    );
    assert_eq!(
        first,
        run(),
        "the same seed + the same request must yield the identical melee injury",
    );
}

// === Draw discipline: a graze takes NO InjuryRng draw. ===

#[test]
fn grazing_melee_strike_takes_no_injury_draw() {
    let (mut app, _attacker, target) = drive_forced_strike(grazing_tuning());

    assert!(
        melee_hits(&app) >= 1,
        "fixture precondition: the strike still CONNECTS (only its severity is a graze)",
    );
    assert!(
        injuries_emitted_for(&app, target).is_empty(),
        "a graze rolls no injury, so no InjuryInflicted is emitted",
    );
    assert!(
        injury_stream_unadvanced(&mut app),
        "a graze must take ZERO InjuryRng draws (the §8 tabling rule)",
    );
}

// === Draw discipline: a fatal wound takes NO InjuryRng draw. ===

#[test]
fn fatal_melee_strike_takes_no_injury_draw() {
    let (mut app, _attacker, target) = drive_forced_strike(fatal_tuning());

    assert!(
        melee_hits(&app) >= 1,
        "fixture precondition: the strike still CONNECTS (its severity is Fatal)",
    );
    assert!(
        injuries_emitted_for(&app, target).is_empty(),
        "a fatal wound is not tabled, so no injury is rolled and none is emitted",
    );
    assert!(
        injury_stream_unadvanced(&mut app),
        "a fatal wound must take ZERO InjuryRng draws (the §8 tabling rule)",
    );
}

// === The positive control for the two zero-draw cases: a WOUND does advance the stream. ===

#[test]
fn wounding_melee_strike_advances_the_injury_stream() {
    let (mut app, _attacker, _target) = drive_forced_strike(wounding_tuning());
    assert!(
        !injury_stream_unadvanced(&mut app),
        "a non-graze, non-fatal melee wound DOES take its one InjuryRng draw — without this \
         control the graze / fatal zero-draw asserts would pass on a dead injury path",
    );
}

// === A miss draws no severity and no injury, and emits nothing. ===

#[test]
fn missed_melee_strike_rolls_no_injury() {
    // A ZERO-Fight attacker against a Fight-positive defender: `atk ≤ def` → the §7 roll is
    // LOST, so the sequence short-circuits before §5/§6/§8.
    let mut app = battle_app_with_tuning(wounding_tuning());
    with_melee_log(&mut app);
    with_injury_log(&mut app);
    install_per_context_injury_content(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([
            defenceless_target(ground(5, 5), PLAYER),
            strong_attacker(ground(6, 5), ENEMY, Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player attacker and one enemy target");
    };
    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    assert_eq!(
        melee_hits(&app),
        0,
        "fixture precondition: the ZERO-Fight attacker MISSES (no MeleeResolved)",
    );
    assert!(
        injuries_emitted_for(&app, target).is_empty(),
        "a missed strike inflicts no injury",
    );
    assert!(
        injury_stream_unadvanced(&mut app),
        "a missed strike must take ZERO InjuryRng draws",
    );
}
