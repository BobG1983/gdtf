use bevy::prelude::Entity;
use gdtf_battle_sim::{
    acts::MeleeRequested,
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::Direction,
    metric::CellLevel,
    prelude::{LifeState, Position},
    test_support::SituationBuilder,
};

use super::harness::*;


#[test]
fn gate_non_adjacent_produces_no_melee() {
    let mut app = battle_app();
    with_melee_log(&mut app);

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

#[test]
fn gate_same_faction_produces_no_melee() {
    let mut app = battle_app();
    with_melee_log(&mut app);

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

#[test]
fn gate_los_blocked_produces_no_melee() {
    let mut app = battle_app();
    with_melee_log(&mut app);

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
