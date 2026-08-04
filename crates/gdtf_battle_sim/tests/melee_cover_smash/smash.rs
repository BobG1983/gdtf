use gdtf_battle_sim::{acts::MeleeRequested, ganger::Direction, test_support::SituationBuilder};

use super::harness::*;

// === C5(a) — a smash on an adjacent Cover cell REDUCES its HP, spends TU, emits MeleeResolved. ===

#[test]
fn smash_reduces_adjacent_cover_hp_and_emits_resolved() {
    let (mut app, seed) = battle_app(0x5508_0A0A);
    with_logs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let cover = ground(6, 5);
    seed_cover(&mut app, cover, 1_000);

    let Some(attacker_entity) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player attacker");
    };
    let (Some(tu_before), Some(hp_before)) = (tu_of(&app, attacker_entity), cover_hp(&app, cover))
    else {
        unreachable!("the attacker carries Tu and the cover cell was seeded");
    };

    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker_entity, cover));
    step(&mut app, 3);

    let Some(hp_after) = cover_hp(&app, cover) else {
        unreachable!("the cover entry persists");
    };
    assert!(
        hp_after < hp_before,
        "C5(a): a melee cover-smash REDUCES the cell's structure HP ({hp_after} < {hp_before})",
    );

    assert!(
        tu_of(&app, attacker_entity).is_some_and(|tu| tu < tu_before),
        "C5(a): the attacker's TU is spent by the smash",
    );

    assert!(
        melee_hits(&app) >= 1,
        "C5(a): a melee cover-smash emits MeleeResolved (the FX signal) — wired end-to-end",
    );
}


#[test]
fn repeated_smashing_destroys_cover_and_fires_the_destroyed_signal() {
    let (mut app, seed) = battle_app(0x5508_0B0B);
    with_logs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let cover = ground(6, 5);
    seed_cover(&mut app, cover, 1);

    let Some(attacker_entity) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player attacker");
    };

    for _ in 0..3 {
        app.world_mut()
            .write_message(MeleeRequested::new_structural(attacker_entity, cover));
        step(&mut app, 3);
    }

    assert!(
        cover_destroyed_flag(&app, cover),
        "C5(b): sufficient melee smashing DESTROYS the cover (its ledger HP reached zero)",
    );
    assert_eq!(
        cover_hp(&app, cover),
        Some(0),
        "C5(b): a destroyed cover's current HP is zero",
    );

    assert!(
        destroyed_hits(&app) >= 1,
        "C5(b): destroying the cover fires the CoverDestroyed signal (the FX bridge)",
    );
    assert!(
        melee_hits(&app) >= 1,
        "C5(b): the smash emits MeleeResolved",
    );
}


#[test]
fn the_structural_smash_is_seed_independent_no_fight_roll() {
    let run_with_seed = |seed: u64| -> Option<u32> {
        let (mut app, seed) = battle_app(seed);
        with_logs(&mut app);
        let situation = SituationBuilder::new()
            .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
            .build_with_gangs();
        drive_setup(&mut app, seed, situation);
        let cover = ground(6, 5);
        seed_cover(&mut app, cover, 1_000);
        let Some(attacker_entity) = player_ganger(&mut app) else {
            unreachable!("setup spawns one player attacker");
        };
        app.world_mut()
            .write_message(MeleeRequested::new_structural(attacker_entity, cover));
        step(&mut app, 3);
        cover_hp(&app, cover)
    };

    let a = run_with_seed(0x0000_1111);
    let b = run_with_seed(0xFFFF_EEEE);
    assert_eq!(
        a, b,
        "C5(c): the structural smash is SEED-INDEPENDENT (no opposed roll, no FightRng draw) — \
         two different seeds leave the same cover HP: {a:?} vs {b:?}",
    );
    assert!(
        a.is_some_and(|hp| hp < 1_000),
        "precondition: the smash actually reduced the cover HP (a real, non-vacuous outcome)",
    );
}


#[test]
fn a_non_adjacent_structure_is_not_smashed() {
    let (mut app, seed) = battle_app(0x5508_0C0C);
    with_logs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let far_cover = ground(8, 5);
    seed_cover(&mut app, far_cover, 1_000);

    let Some(attacker_entity) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player attacker");
    };
    let Some(hp_before) = cover_hp(&app, far_cover) else {
        unreachable!("the far cover cell was seeded");
    };

    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker_entity, far_cover));
    step(&mut app, 3);

    assert_eq!(
        cover_hp(&app, far_cover),
        Some(hp_before),
        "the 8-adjacency gate held — a non-adjacent structure takes NO melee damage",
    );
    assert_eq!(
        melee_hits(&app),
        0,
        "a non-adjacent structure smash emits NO MeleeResolved",
    );
}
