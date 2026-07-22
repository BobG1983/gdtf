//! The structural smash act — reduces adjacent cover HP + emits resolved, destroys + fires
//! the destroyed signal on depletion, takes no seed-dependent roll, and gates on adjacency.

use gdtf_battle_sim::{acts::MeleeRequested, ganger::Direction, test_support::SituationBuilder};

use super::harness::*;

// === C5(a) — a smash on an adjacent Cover cell REDUCES its HP, spends TU, emits MeleeResolved. ===

#[test]
fn smash_reduces_adjacent_cover_hp_and_emits_resolved() {
    let (mut app, seed) = battle_app(0x5508_0A0A);
    with_logs(&mut app);

    // The player attacker faces East at (5,5); the cover cell is directly east at (6,5), 8-adjacent.
    let situation = SituationBuilder::new()
        .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    // A durable adjacent wall — big enough that ONE smash damages but does not destroy it (so this
    // isolates the "HP reduced" clause from the "destroyed" clause).
    let cover = ground(6, 5);
    seed_cover(&mut app, cover, 1_000);

    let Some(attacker_entity) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player attacker");
    };
    let (Some(tu_before), Some(hp_before)) = (tu_of(&app, attacker_entity), cover_hp(&app, cover))
    else {
        unreachable!("the attacker carries Tu and the cover cell was seeded");
    };

    // Drive the smash THROUGH the buffered MeleeRequested::new_structural (the structural form
    // the input layer writes).
    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker_entity, cover));
    step(&mut app, 3);

    // C5(a): the cover's HP went DOWN through the REAL ledger.
    let Some(hp_after) = cover_hp(&app, cover) else {
        unreachable!("the cover entry persists");
    };
    assert!(
        hp_after < hp_before,
        "C5(a): a melee cover-smash REDUCES the cell's structure HP ({hp_after} < {hp_before})",
    );

    // C5(a): the attacker's TU was spent (a swing at a structure costs TU like a swing at a ganger).
    assert!(
        tu_of(&app, attacker_entity).is_some_and(|tu| tu < tu_before),
        "C5(a): the attacker's TU is spent by the smash",
    );

    // C5(a): a MeleeResolved (the strike-glyph FX signal) was emitted — the structural act
    // resolved end-to-end on the real runtime path. PIN-DISCRIMINATING (fails if unwired).
    assert!(
        melee_hits(&app) >= 1,
        "C5(a): a melee cover-smash emits MeleeResolved (the FX signal) — wired end-to-end",
    );
}

// === C5(b) — sufficient/repeated smashing DESTROYS the cover and fires the cover-destroyed signal. ===

#[test]
fn repeated_smashing_destroys_cover_and_fires_the_destroyed_signal() {
    let (mut app, seed) = battle_app(0x5508_0B0B);
    with_logs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    // A FRAGILE adjacent wall (1 HP, armorless) so a single connecting smash depletes it to zero
    // — no pinned magnitude, just "at least one smash's damage ≥ 1 HP" (the armorless fists deal
    // real damage). Structuring it this way keeps the destroy clause magnitude-free.
    let cover = ground(6, 5);
    seed_cover(&mut app, cover, 1);

    let Some(attacker_entity) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player attacker");
    };

    // Smash repeatedly — each swing re-tops the attacker's TU is NOT modelled here, but the
    // structural depletion is CUMULATIVE across strikes (the ledger persists current_hp), and a
    // fragile 1-HP wall is destroyed on the first connecting hit. Drive a few to be robust to the
    // TU pool (a real fielded attacker has ample TU for several fists swings).
    for _ in 0..3 {
        app.world_mut()
            .write_message(MeleeRequested::new_structural(attacker_entity, cover));
        step(&mut app, 3);
    }

    // C5(b): the cover was destroyed (HP depleted to zero) — read the model ledger's destroyed flag.
    assert!(
        cover_destroyed_flag(&app, cover),
        "C5(b): sufficient melee smashing DESTROYS the cover (its ledger HP reached zero)",
    );
    assert_eq!(
        cover_hp(&app, cover),
        Some(0),
        "C5(b): a destroyed cover's current HP is zero",
    );

    // C5(b): the EXISTING CoverDestroyed signal fired (the presenter's GTW-386 FX reacts to it).
    // PIN-DISCRIMINATING: with the cover-destroyed bridge unwired this would be zero.
    assert!(
        destroyed_hits(&app) >= 1,
        "C5(b): destroying the cover fires the CoverDestroyed signal (the GTW-386 FX bridge)",
    );
    // And a MeleeResolved was emitted for the smash too (the strike-glyph FX).
    assert!(
        melee_hits(&app) >= 1,
        "C5(b): the smash emits MeleeResolved",
    );
}

// === C5(c) — the structural path takes NO FightRng draw: the outcome is SEED-INDEPENDENT. ===

#[test]
fn the_structural_smash_is_seed_independent_no_fight_roll() {
    // Resolve the same smash on the same durable wall under TWO DIFFERENT battle seeds and read
    // the resulting cover HP. A contested (opposed-Fight) path would draw from FightRng and vary
    // with the seed; the UNCONTESTED structural path takes NO RNG draw, so the HP-after is
    // IDENTICAL across seeds — the C5(c) "no FightRng draw" property.
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
    // And it's a non-trivial outcome (the smash actually removed HP under both seeds — else the
    // seed-independence is vacuous).
    assert!(
        a.is_some_and(|hp| hp < 1_000),
        "precondition: the smash actually reduced the cover HP (a real, non-vacuous outcome)",
    );
}

// === A gate check: a NON-ADJACENT structure cell produces no smash (the 8-adjacency gate). ===

#[test]
fn a_non_adjacent_structure_is_not_smashed() {
    let (mut app, seed) = battle_app(0x5508_0C0C);
    with_logs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([attacker(ground(5, 5), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    // A wall THREE cells east — well outside the 8-adjacent reach.
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
