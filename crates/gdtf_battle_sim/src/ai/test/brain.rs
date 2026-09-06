use super::support::{
    ENEMY, PLAYER, active_of, brain_app, drain_fires, drain_moves, give_disabled_hand, ground,
    place_occupant, spawn_combatant, spawn_combatant_handed, tu_of,
};
use crate::{armor::BodyPart, ganger::Direction, metric::Cell, weapon::Handedness};

const FRAME_CAP: usize = 80;

#[test]
fn enemy_that_can_see_fires_then_the_turn_returns_to_the_player() {
    let mut app = brain_app();
    let player_at = ground(8, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        100,
        2,
    );
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 2);
    place_occupant(&mut app, player_at, player);

    let mut fires = Vec::new();
    let mut returned = false;
    for _ in 0..FRAME_CAP {
        app.update();
        fires.extend(drain_fires(&mut app));
        if active_of(&app) == PLAYER {
            returned = true;
            break;
        }
    }

    assert!(
        fires.iter().any(|f| f.shooter == enemy
            && f.target_cell == Cell::new(8, 5)
            && *f.target_level == 0),
        "the enemy must emit a FireRequested at the player's (8,5,0) cell: {fires:?}",
    );
    assert!(
        returned,
        "the enemy turn must terminate and hand control back to the player within the cap",
    );
    assert!(
        tu_of(&app, enemy) < 100,
        "the enemy spent TU within its budget (it fired / acted): {}",
        tu_of(&app, enemy),
    );
}

#[test]
fn enemy_that_cannot_see_advances_toward_contact() {
    let mut app = brain_app();
    let player_at = ground(40, 5);
    let enemy = spawn_combatant(app.world_mut(), ground(2, 5), ENEMY, Direction::East, 20, 6);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);

    let mut moves = Vec::new();
    let mut fires = Vec::new();
    let mut returned = false;
    for _ in 0..FRAME_CAP {
        app.update();
        moves.extend(drain_moves(&mut app));
        fires.extend(drain_fires(&mut app));
        if active_of(&app) == PLAYER {
            returned = true;
            break;
        }
    }

    assert!(
        moves.iter().any(|m| m.actor == enemy && m.dest.x > 2),
        "the enemy must emit a MoveRequested stepping toward the player (east of x=2): {moves:?}",
    );
    assert!(
        fires.is_empty(),
        "an enemy that cannot see the player must never fire: {fires:?}",
    );
    assert!(
        returned,
        "the advance turn must terminate and hand control back to the player within the cap",
    );
    assert!(
        tu_of(&app, enemy) < 20,
        "the enemy spent TU advancing within its budget: {}",
        tu_of(&app, enemy),
    );
}

#[test]
fn enemy_fires_the_opposing_player_and_never_a_friendly_enemy() {
    let mut app = brain_app();
    // Nearer the shooter than the player is, and later in act order, so the shooter fires first.
    let friend_at = ground(4, 7);
    let player_at = ground(8, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        100,
        6,
    );
    let friend = spawn_combatant(app.world_mut(), friend_at, ENEMY, Direction::East, 100, 6);
    place_occupant(&mut app, friend_at, friend);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);

    let mut fires = Vec::new();
    for _ in 0..FRAME_CAP {
        app.update();
        fires.extend(drain_fires(&mut app));
        if active_of(&app) == PLAYER {
            break;
        }
    }

    assert!(
        fires.iter().any(|f| f.shooter == enemy
            && f.target_cell == Cell::new(8, 5)
            && *f.target_level == 0),
        "the enemy must fire the opposing player at its (8,5,0) cell: {fires:?}",
    );
    assert!(
        !fires.iter().any(|f| f.target_cell == Cell::new(4, 7)),
        "the brain must never target a friendly enemy (no shot at (4,7)): {fires:?}",
    );
    assert_ne!(
        friend, player,
        "the friendly enemy and the player must be distinct entities",
    );
}

#[test]
fn enemy_brain_never_drives_a_player_unit() {
    let mut app = brain_app();
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        100,
        6,
    );
    let player_near_at = ground(6, 5);
    let player_near = spawn_combatant(
        app.world_mut(),
        player_near_at,
        PLAYER,
        Direction::West,
        100,
        6,
    );
    place_occupant(&mut app, player_near_at, player_near);
    let player_far_at = ground(8, 5);
    let player_far = spawn_combatant(
        app.world_mut(),
        player_far_at,
        PLAYER,
        Direction::West,
        100,
        6,
    );
    place_occupant(&mut app, player_far_at, player_far);

    let mut fires = Vec::new();
    let mut moves = Vec::new();
    for _ in 0..FRAME_CAP {
        app.update();
        fires.extend(drain_fires(&mut app));
        moves.extend(drain_moves(&mut app));
        if active_of(&app) == PLAYER {
            break;
        }
    }

    assert!(
        !fires
            .iter()
            .any(|f| f.shooter == player_near || f.shooter == player_far),
        "the brain must never drive a player as a shooter on the enemy turn: {fires:?}",
    );
    assert!(
        !moves
            .iter()
            .any(|m| m.actor == player_near || m.actor == player_far),
        "the brain must never drive a player as a mover on the enemy turn: {moves:?}",
    );
    assert!(
        fires.iter().any(|f| f.shooter == enemy
            && f.target_cell == Cell::new(6, 5)
            && *f.target_level == 0),
        "the enemy must engage the nearer player at (6,5,0): {fires:?}",
    );
}

#[test]
fn ai_does_not_engage_two_handed_weapon_below_two_hands() {
    let mut app = brain_app();
    let player_at = ground(8, 5);
    let enemy = spawn_combatant_handed(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        Handedness::TwoHanded,
    );
    give_disabled_hand(app.world_mut(), enemy, BodyPart::RightArm);
    let player = spawn_combatant_handed(
        app.world_mut(),
        player_at,
        PLAYER,
        Direction::West,
        Handedness::OneHanded,
    );
    place_occupant(&mut app, player_at, player);

    let mut fires = Vec::new();
    let mut returned = false;
    for _ in 0..FRAME_CAP {
        app.update();
        fires.extend(drain_fires(&mut app));
        if active_of(&app) == PLAYER {
            returned = true;
            break;
        }
    }

    assert!(
        fires.is_empty(),
        "the AI must NOT engage a TwoHanded weapon below two hands (shared can_fire gate): {fires:?}",
    );
    assert!(
        returned,
        "the enemy turn must still terminate within the cap (advance/hold, then end-turn)",
    );
}

#[test]
fn ai_engages_two_handed_weapon_with_two_hands() {
    let mut app = brain_app();
    let player_at = ground(8, 5);
    let enemy = spawn_combatant_handed(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        Handedness::TwoHanded,
    );
    let player = spawn_combatant_handed(
        app.world_mut(),
        player_at,
        PLAYER,
        Direction::West,
        Handedness::OneHanded,
    );
    place_occupant(&mut app, player_at, player);

    let mut fires = Vec::new();
    for _ in 0..FRAME_CAP {
        app.update();
        fires.extend(drain_fires(&mut app));
        if active_of(&app) == PLAYER {
            break;
        }
    }

    assert!(
        fires.iter().any(|f| f.shooter == enemy
            && f.target_cell == Cell::new(8, 5)
            && *f.target_level == 0),
        "with two hands the AI engages the TwoHanded weapon at the player's (8,5,0) cell: {fires:?}",
    );
    assert!(
        tu_of(&app, enemy) < 100,
        "the enemy spent TU firing: {}",
        tu_of(&app, enemy),
    );
}
