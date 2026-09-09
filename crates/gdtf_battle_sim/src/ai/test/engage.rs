use bevy::prelude::Entity;

use super::support::{
    ENEMY, FireRequested, LifeState, PLAYER, SetAimingRequested, brain_app, drain_aims,
    drain_fires, drive_until_player_turn, ground, place_occupant, spawn_combatant,
};
use crate::{ganger::Direction, metric::Cell};

struct Engaged {
    enemy: Entity,
    fires: Vec<FireRequested>,
    aims:  Vec<SetAimingRequested>,
}

// One enemy at (2,5) facing East, one player at (8,5) carrying the given life state.
fn drive_against_lone_target(life: LifeState) -> Engaged {
    let mut app = brain_app();
    let player_at = ground(8, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        100,
        6,
    );
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);
    app.world_mut().entity_mut(player).insert(life);

    let mut fires = Vec::new();
    let mut aims = Vec::new();
    drive_until_player_turn(&mut app, |app| {
        fires.extend(drain_fires(app));
        aims.extend(drain_aims(app));
    });
    Engaged { enemy, fires, aims }
}

#[test]
fn a_dead_target_is_never_aimed_at_or_fired_at() {
    let driven = drive_against_lone_target(LifeState::Dead);

    assert!(
        !driven.fires.iter().any(|fire| fire.shooter == driven.enemy),
        "a dead target must leave the fire set, so the enemy fires nothing: {:?}",
        driven.fires,
    );
    assert!(
        !driven.aims.iter().any(|aim| aim.actor == driven.enemy),
        "a dead target must leave the fire set, so the enemy never aims on: {:?}",
        driven.aims,
    );
}

#[test]
fn a_downed_target_is_never_aimed_at_or_fired_at() {
    let driven = drive_against_lone_target(LifeState::Downed);

    assert!(
        !driven.fires.iter().any(|fire| fire.shooter == driven.enemy),
        "a downed target must leave the fire set, so the enemy fires nothing: {:?}",
        driven.fires,
    );
    assert!(
        !driven.aims.iter().any(|aim| aim.actor == driven.enemy),
        "a downed target must leave the fire set, so the enemy never aims on: {:?}",
        driven.aims,
    );
}

#[test]
fn a_live_enemy_is_fired_at_over_a_nearer_body() {
    let mut app = brain_app();
    let body_at = ground(4, 7);
    let live_at = ground(8, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        100,
        6,
    );
    let body = spawn_combatant(app.world_mut(), body_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, body_at, body);
    let live = spawn_combatant(app.world_mut(), live_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, live_at, live);
    app.world_mut().entity_mut(body).insert(LifeState::Dead);

    let mut fires = Vec::new();
    drive_until_player_turn(&mut app, |app| fires.extend(drain_fires(app)));

    let cells: Vec<Cell> = fires
        .iter()
        .filter(|fire| fire.shooter == enemy)
        .map(|fire| fire.target_cell)
        .collect();
    assert!(
        !cells.is_empty(),
        "the live player at (8,5) is engageable, so the enemy must fire: {fires:?}",
    );
    assert!(
        cells.iter().all(|cell| *cell == Cell::new(8, 5)),
        "every shot must go at the live player at (8,5), not the nearer body at (4,7): {cells:?}",
    );
}
