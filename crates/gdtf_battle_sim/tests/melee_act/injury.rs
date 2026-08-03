use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    acts::MeleeRequested,
    ganger::Direction,
    rng::{BattleSeed, InjuryRng},
    test_support::SituationBuilder,
    tuning::CombatTuning,
};

use super::{harness::*, injury_content::*};

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

fn injury_stream_unadvanced(app: &mut App) -> bool {
    let mut reference = InjuryRng::from_root(BattleSeed::new(SEED));
    let expected = reference.next_u64();
    let mut live = app.world_mut().resource_mut::<InjuryRng>();
    live.next_u64() == expected
}


#[test]
fn connecting_melee_wound_rolls_a_melee_table_injury_onto_the_ledger() {
    let (app, _attacker, target) = drive_forced_strike(wounding_tuning());

    assert!(
        melee_hits(&app) >= 1,
        "fixture precondition: the forced strike connects (a MeleeResolved was emitted)",
    );

    let emitted = injuries_emitted_for(&app, target);
    assert!(
        !emitted.is_empty(),
        "a non-graze, non-fatal melee wound must emit an InjuryInflicted for the target",
    );

    assert!(
        emitted.iter().all(|name| name == MELEE_ONLY),
        "the melee wound must roll from the MELEE weighting table (expected only \
         {MELEE_ONLY:?}, got {emitted:?} — {OTHER_ONLY:?} means the wrong context was sampled)",
    );

    let ledger = ledger_names(&app, target);
    assert!(
        ledger.iter().any(|name| name == MELEE_ONLY),
        "the rolled melee injury must land on the target's InflictedInjuries ledger, got \
         {ledger:?}",
    );
}


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


#[test]
fn wounding_melee_strike_advances_the_injury_stream() {
    let (mut app, _attacker, _target) = drive_forced_strike(wounding_tuning());
    assert!(
        !injury_stream_unadvanced(&mut app),
        "a non-graze, non-fatal melee wound DOES take its one InjuryRng draw — without this \
         control the graze / fatal zero-draw asserts would pass on a dead injury path",
    );
}


#[test]
fn missed_melee_strike_rolls_no_injury() {
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
