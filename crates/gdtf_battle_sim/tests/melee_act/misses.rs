//! C6(b) + determinism — the whiff: a miss spends TU but applies nothing, and the strike
//! outcome is deterministic across identical runs.

use gdtf_battle_sim::{
    acts::MeleeRequested,
    ganger::{Cool, Direction, Facing, Grit, Speed, Strength, Toughness},
    metric::CellLevel,
    prelude::{Faction, Stance, StanceKind},
    test_support::{GangerSpawnBuilder, SituationBuilder},
};

use super::harness::*;

/// A Fight-positive target (some Strength / Speed) so a ZERO-Fight attacker's strike does NOT
/// connect under variance 0 (`atk == 0 ≤ def > 0`) — the miss case's defender.
fn fighting_target(at: CellLevel, faction: u8) -> gdtf_battle_sim::situation::GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .toughness(Toughness::new(120.0))
        .build()
}

/// A ZERO-Fight attacker (every Fight contributor 0) so under variance 0 its rolled Fight is 0
/// and a strike against any Fight-positive defender is a MISS (`atk ≤ def`).
fn weak_attacker(
    at: CellLevel,
    faction: u8,
    facing: Direction,
) -> gdtf_battle_sim::situation::GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(0.0))
        .strength(Strength::new(0.0))
        .grit(Grit::new(0.0))
        .cool(Cool::new(0.0))
        .build()
}

// === C6(b) — a miss (the opposed roll lost) applies NO damage, records no Wound, emits no
// MeleeResolved — yet the attacker's TU IS spent (a swing costs TU regardless). ===

#[test]
fn miss_applies_no_damage_but_spends_tu() {
    let mut app = battle_app();
    with_melee_log(&mut app);

    // A ZERO-Fight player attacker faces East at (5,5); a Fight-positive enemy stands 8-adjacent
    // at (6,5). Under variance 0, atk(0) ≤ def(>0) → the opposed roll is LOST (a miss).
    let situation = SituationBuilder::new()
        .with_gangers([
            weak_attacker(ground(5, 5), PLAYER, Direction::East),
            fighting_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player attacker and one enemy target");
    };
    let (Some(attacker_tu_before), Some(target_hp_before), Some(target_wounds_before)) = (
        tu_of(&app, attacker),
        hp_of(&app, target),
        wounds_of(&app, target),
    ) else {
        unreachable!("both gangers carry Tu / Hp / Wounds pools");
    };

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    // C6(b): NO damage — HP unchanged, Wounds unchanged.
    assert_eq!(
        hp_of(&app, target),
        Some(target_hp_before),
        "C6(b): a missed strike (opposed roll lost) takes NO HP",
    );
    assert_eq!(
        wounds_of(&app, target),
        Some(target_wounds_before),
        "C6(b): a missed strike records NO Wound",
    );
    // C6(b): NO MeleeResolved (the §7 connect gate held — a miss emits nothing).
    assert_eq!(
        melee_hits(&app),
        0,
        "C6(b): a missed strike emits NO MeleeResolved",
    );
    // C6(b): the swing STILL cost TU (a swing costs TU whether or not it connects).
    assert!(
        tu_of(&app, attacker).is_some_and(|tu| tu < attacker_tu_before),
        "C6(b): the attacker's TU is spent even on a miss (a swing costs TU regardless)",
    );
}

// === A defensive determinism check: the same seed + tuning reproduces the same connect outcome
// (HP after) through the live path. ===

#[test]
fn the_strike_outcome_is_deterministic_across_identical_runs() {
    let run_once = || {
        let mut app = battle_app();
        with_melee_log(&mut app);
        let situation = SituationBuilder::new()
            .with_gangers([
                strong_attacker(ground(5, 5), PLAYER, Direction::East),
                defenceless_target(ground(6, 5), ENEMY),
            ])
            .build_with_gangs();
        drive_setup(&mut app, situation);
        let (Some(attacker), Some(target)) =
            (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
        else {
            unreachable!("setup spawns one player and one enemy");
        };
        app.world_mut()
            .write_message(MeleeRequested::new(attacker, target));
        step(&mut app, 3);
        (hp_of(&app, target), melee_hits(&app))
    };

    let first = run_once();
    let second = run_once();
    assert_eq!(
        first, second,
        "the same seed + tuning reproduces the same strike outcome (replay-stable through the \
         live path)",
    );
    // And it's a non-trivial outcome (the strike actually connected — else determinism is vacuous).
    assert!(
        first.1 >= 1,
        "precondition: the deterministic run actually connects (variance 0 + a zero-Fight target)",
    );
}
