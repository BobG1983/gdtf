//! The brain emits the two downed acts, and refuses them when the pool cannot cover the cost.

use super::support::*;

// A pool that walks a few cells and runs out well short of the downed target.
const WALK_POOL: u8 = 20;

#[test]
fn an_adjacent_downed_foe_is_executed_and_the_turn_ends() {
    let mut app = brain_app();
    let pool = execute_cost(&app).saturating_mul(2);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(5, 5),
        ENEMY,
        Direction::East,
        pool,
        6,
    );
    let downed_at = ground(6, 5);
    let player = spawn_combatant(app.world_mut(), downed_at, PLAYER, Direction::West, 100, 2);
    place_occupant(&mut app, downed_at, player);
    down_before_the_enemy_turn(&mut app, &[player]);

    let drive = drive_until_player(&mut app);

    assert_eq!(
        drive.executes_for(enemy, player).len(),
        1,
        "the brain must emit exactly one execute for the adjacent downed foe: {:?}",
        drive.executes,
    );
    assert_eq!(
        life_of(&app, player),
        Some(LifeState::Dead),
        "the real dispatch must kill the executed target on the frame the brain wrote it",
    );
}

#[test]
fn an_adjacent_bleeding_ally_is_stabilized_once_and_the_turn_ends() {
    let mut app = brain_app();
    let pool = stabilize_cost(&app).saturating_mul(2);
    let medic = spawn_combatant(
        app.world_mut(),
        ground(5, 5),
        ENEMY,
        Direction::East,
        pool,
        6,
    );
    let ally_at = ground(6, 5);
    let ally = spawn_combatant(app.world_mut(), ally_at, ENEMY, Direction::West, 100, 2);
    place_occupant(&mut app, ally_at, ally);
    down_before_the_enemy_turn(&mut app, &[ally]);

    assert!(
        is_bleeding(&app, ally),
        "mark_downed_bleeding must be what puts BleedingOut on the downed ally",
    );

    let drive = drive_until_player(&mut app);

    assert_eq!(
        drive.stabilizes_for(medic, ally).len(),
        1,
        "the brain must emit exactly one stabilize across the whole drive: {:?}",
        drive.stabilizes,
    );
    assert!(
        !is_bleeding(&app, ally),
        "the real dispatch must stop the ally bleeding",
    );
}

#[test]
fn a_downed_foe_out_of_reach_is_walked_toward_and_never_executed() {
    let mut app = brain_app();
    let start_x = 2;
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(start_x, 5),
        ENEMY,
        Direction::East,
        WALK_POOL,
        6,
    );
    let downed_at = ground(40, 5);
    let player = spawn_combatant(app.world_mut(), downed_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, downed_at, player);
    down_before_the_enemy_turn(&mut app, &[player]);

    let drive = drive_until_player(&mut app);

    assert!(
        drive.executes.is_empty(),
        "a downed foe out of reach must not be executed: {:?}",
        drive.executes,
    );
    assert!(
        drive.stabilizes.is_empty(),
        "there is no downed ally to stabilize: {:?}",
        drive.stabilizes,
    );
    assert!(
        drive
            .moves
            .iter()
            .any(|step| step.actor == enemy && step.dest.x > start_x),
        "the enemy must still walk toward the downed foe: {:?}",
        drive.moves,
    );
}

#[test]
fn an_unaffordable_execute_is_not_requested_and_the_turn_ends() {
    let mut app = brain_app();
    let pool = execute_cost(&app).saturating_sub(1);
    let _enemy = spawn_combatant(
        app.world_mut(),
        ground(5, 5),
        ENEMY,
        Direction::East,
        pool,
        6,
    );
    let downed_at = ground(6, 5);
    let player = spawn_combatant(app.world_mut(), downed_at, PLAYER, Direction::West, 100, 2);
    place_occupant(&mut app, downed_at, player);
    down_before_the_enemy_turn(&mut app, &[player]);

    assert_eq!(
        life_of(&app, player),
        Some(LifeState::Downed),
        "every other term of can_execute must pass, so only the TU term can refuse this one",
    );

    let drive = drive_until_player(&mut app);

    assert!(
        drive.executes.is_empty(),
        "an execute costing more TU than the actor has must never be requested: {:?}",
        drive.executes,
    );
}

#[test]
fn an_unaffordable_stabilize_is_not_requested_and_the_turn_ends() {
    let mut app = brain_app();
    let pool = stabilize_cost(&app).saturating_sub(1);
    let _medic = spawn_combatant(
        app.world_mut(),
        ground(5, 5),
        ENEMY,
        Direction::East,
        pool,
        6,
    );
    let ally_at = ground(6, 5);
    let ally = spawn_combatant(app.world_mut(), ally_at, ENEMY, Direction::West, 100, 2);
    place_occupant(&mut app, ally_at, ally);
    down_before_the_enemy_turn(&mut app, &[ally]);

    assert!(
        is_bleeding(&app, ally),
        "mark_downed_bleeding must be what puts BleedingOut on the downed ally",
    );

    let drive = drive_until_player(&mut app);

    assert!(
        drive.stabilizes.is_empty(),
        "a stabilize costing more TU than the actor has must never be requested: {:?}",
        drive.stabilizes,
    );
    assert!(
        is_bleeding(&app, ally),
        "a refused stabilize leaves the ally bleeding",
    );
}
