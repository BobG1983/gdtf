use bevy::prelude::Entity;

use super::support::{
    ENEMY, LifeState, MoveRequested, PLAYER, brain_app, drain_moves, drive_until_player_turn,
    ground, place_occupant, spawn_combatant,
};
use crate::ganger::Direction;

struct Advanced {
    enemy: Entity,
    moves: Vec<MoveRequested>,
}

// One enemy at (2,5) facing East, one player at (40,5) out of sight carrying the given life state.
fn drive_toward_lone_target(life: LifeState) -> Advanced {
    let mut app = brain_app();
    let player_at = ground(40, 5);
    let enemy = spawn_combatant(app.world_mut(), ground(2, 5), ENEMY, Direction::East, 20, 6);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);
    app.world_mut().entity_mut(player).insert(life);

    let mut moves = Vec::new();
    drive_until_player_turn(&mut app, |app| moves.extend(drain_moves(app)));
    Advanced { enemy, moves }
}

#[test]
fn a_corpse_is_never_walked_toward() {
    let driven = drive_toward_lone_target(LifeState::Dead);

    assert!(
        !driven.moves.iter().any(|step| step.actor == driven.enemy),
        "a dead target must leave the advance list, so the enemy never steps at it: {:?}",
        driven.moves,
    );
}

#[test]
fn a_downed_enemy_is_still_walked_toward() {
    let driven = drive_toward_lone_target(LifeState::Downed);

    assert!(
        driven
            .moves
            .iter()
            .any(|step| step.actor == driven.enemy && step.dest.x > 2),
        "a downed target stays a destination, so the enemy must step east of x=2: {:?}",
        driven.moves,
    );
}
