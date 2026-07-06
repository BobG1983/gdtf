//! C6(c) — the melee act gates: adjacency, faction, dead target, and LOS each reject the
//! strike (no HP loss, no `MeleeResolved`).

use bevy::prelude::Entity;
use gdtf_battle_sim::{
    LifeState, Position,
    acts::MeleeRequested,
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::Direction,
    metric::CellLevel,
    test_support::SituationBuilder,
};

use super::harness::*;

// === C6(c) — the GATES. Each discriminating case (non-adjacent / LOS-blocked / non-enemy /
// dead) produces NO melee: no HP loss, no MeleeResolved. ===

/// C6(c) gate 1 — a NON-ADJACENT target (Chebyshev > 1) produces no melee.
#[test]
fn gate_non_adjacent_produces_no_melee() {
    let mut app = battle_app();
    with_melee_log(&mut app);

    // The enemy stands THREE cells east — well outside the 8-adjacent reach.
    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(8, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player and one enemy");
    };
    let Some(hp_before) = hp_of(&app, target) else {
        unreachable!("the target has Hp");
    };

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    assert_eq!(
        hp_of(&app, target),
        Some(hp_before),
        "C6(c): a non-adjacent target takes NO melee damage (the 8-adjacency gate held)",
    );
    assert_eq!(
        melee_hits(&app),
        0,
        "C6(c): a non-adjacent strike emits NO MeleeResolved",
    );
}

/// C6(c) gate 2 — a SAME-FACTION (ally) target produces no melee (no friendly melee).
#[test]
fn gate_same_faction_produces_no_melee() {
    let mut app = battle_app();
    with_melee_log(&mut app);

    // BOTH gangers are PLAYER faction — an ally is never a melee target.
    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), PLAYER),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let world = app.world_mut();
    let mut q = world.query::<(Entity, &Position)>();
    let mut entities: Vec<(Entity, CellLevel)> = q.iter(world).map(|(e, p)| (e, **p)).collect();
    entities.sort_by_key(|(_, p)| (p.z, p.y, p.x));
    let (Some(&(attacker, _)), Some(&(target, _))) = (entities.first(), entities.get(1)) else {
        unreachable!("setup spawns two player gangers");
    };
    let Some(hp_before) = hp_of(&app, target) else {
        unreachable!("the target has Hp");
    };

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    assert_eq!(
        hp_of(&app, target),
        Some(hp_before),
        "C6(c): a same-faction ally takes NO melee damage (the opposing-faction gate held)",
    );
    assert_eq!(
        melee_hits(&app),
        0,
        "C6(c): a same-faction strike emits NO MeleeResolved",
    );
}

/// C6(c) gate 3 — a DEAD target produces no melee (only an alive opposing ganger is a target).
#[test]
fn gate_dead_target_produces_no_melee() {
    let mut app = battle_app();
    with_melee_log(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player and one enemy");
    };
    // Force the target DEAD (a corpse) before the strike — only an alive target is meleeable.
    app.world_mut().entity_mut(target).insert(LifeState::Dead);
    app.update();
    let Some(hp_before) = hp_of(&app, target) else {
        unreachable!("the target persists");
    };

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    assert_eq!(
        hp_of(&app, target),
        Some(hp_before),
        "C6(c): a DEAD target takes NO melee damage (the alive gate held)",
    );
    assert_eq!(
        melee_hits(&app),
        0,
        "C6(c): a strike on a dead target emits NO MeleeResolved",
    );
}

/// C6(c) gate 4 — a LOS-BLOCKED target produces no melee. The attacker and target are
/// DIAGONALLY 8-adjacent (Chebyshev 1), and BOTH orthogonal corner cells the diagonal sight ray
/// can cross are filled with a HIGH wall — so whichever corner the voxel-DDA steps through, the
/// center-to-center sight march stops on a wall BEFORE the target. Adjacency + faction + alive
/// all hold; only the LOS gate rejects the strike (so this is discriminating: with the walls
/// REMOVED the same geometry connects — proven by the other connect tests at the same range).
#[test]
fn gate_los_blocked_produces_no_melee() {
    let mut app = battle_app();
    with_melee_log(&mut app);

    // Attacker at (5,5); target DIAGONALLY 8-adjacent at (6,6). The diagonal sight ray crosses
    // one of the two corner cells (6,5) / (5,6) — wall BOTH so it is blocked either way.
    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 6), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player and one enemy");
    };
    let Some(hp_before) = hp_of(&app, target) else {
        unreachable!("the target has Hp");
    };

    // Seed HIGH walls in BOTH corner cells the diagonal sight ray can cross, so the march stops
    // on a wall before the target regardless of the DDA's axis-step tie-break. The cover ledger
    // is the model surface `has_los` marches; seeding it directly is the test idiom.
    {
        let mut ledger = app.world_mut().resource_mut::<CoverLedger>();
        for corner in [ground(6, 5), ground(5, 6)] {
            let wall = CoverEntry::seeded(
                CoverHp::new(100),
                HeightBand::High,
                ArmorProtection::new(50),
                ArmorHardness::new(50),
            );
            ledger.insert(corner, wall);
        }
    }
    app.update();

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    assert_eq!(
        hp_of(&app, target),
        Some(hp_before),
        "C6(c): a LOS-blocked target takes NO melee damage (the LOS gate held)",
    );
    assert_eq!(
        melee_hits(&app),
        0,
        "C6(c): a LOS-blocked strike emits NO MeleeResolved",
    );
}
