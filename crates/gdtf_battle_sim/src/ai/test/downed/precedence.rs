//! Which act the downed arm pre-empts, and which row it picks when several pass one gate.

use super::support::*;

// On top of the execute, enough pool for the aimed shot at the live foe that follows it.
const ENGAGE_POOL: u8 = 40;

// One actor with exactly one execute in the pool, and two 8-adjacent downed foes.
struct TwoBodies {
    actor: Entity,
    low:   Entity,
    high:  Entity,
    drive: DownedDrive,
}

// Both bodies sit at Chebyshev distance 1, so `pick_nearest` decides on cell order and low y wins.
fn drive_two_adjacent_bodies(spawn_low_first: bool) -> TwoBodies {
    let mut app = brain_app();
    let pool = execute_cost(&app);
    let actor = spawn_combatant(
        app.world_mut(),
        ground(5, 5),
        ENEMY,
        Direction::East,
        pool,
        6,
    );
    let low_at = ground(6, 4);
    let high_at = ground(6, 6);
    let (low, high) = if spawn_low_first {
        let low = spawn_combatant(app.world_mut(), low_at, PLAYER, Direction::West, 100, 2);
        let high = spawn_combatant(app.world_mut(), high_at, PLAYER, Direction::West, 100, 2);
        (low, high)
    } else {
        let high = spawn_combatant(app.world_mut(), high_at, PLAYER, Direction::West, 100, 2);
        let low = spawn_combatant(app.world_mut(), low_at, PLAYER, Direction::West, 100, 2);
        (low, high)
    };
    place_occupant(&mut app, low_at, low);
    place_occupant(&mut app, high_at, high);
    down_before_the_enemy_turn(&mut app, &[low, high]);

    let drive = drive_until_player(&mut app);
    TwoBodies {
        actor,
        low,
        high,
        drive,
    }
}

#[test]
fn execute_is_requested_a_frame_before_stabilize_when_both_are_adjacent() {
    let mut app = brain_app();
    let pool = execute_cost(&app)
        .saturating_add(stabilize_cost(&app))
        .saturating_mul(2);
    let actor = spawn_combatant(
        app.world_mut(),
        ground(5, 5),
        ENEMY,
        Direction::East,
        pool,
        6,
    );
    let foe_at = ground(6, 5);
    let ally_at = ground(4, 5);
    let foe = spawn_combatant(app.world_mut(), foe_at, PLAYER, Direction::West, 100, 2);
    let ally = spawn_combatant(app.world_mut(), ally_at, ENEMY, Direction::East, 100, 2);
    place_occupant(&mut app, foe_at, foe);
    place_occupant(&mut app, ally_at, ally);
    down_before_the_enemy_turn(&mut app, &[foe, ally]);

    let drive = drive_until_player(&mut app);

    let executed_on = drive.executes_for(actor, foe).into_iter().min();
    let stabilized_on = drive.stabilizes_for(actor, ally).into_iter().min();
    assert!(
        matches!(
            (executed_on, stabilized_on),
            (Some(execute), Some(stabilize)) if execute < stabilize
        ),
        "execute must be requested on a strictly earlier frame than stabilize: \
         execute {executed_on:?}, stabilize {stabilized_on:?}",
    );
}

#[test]
fn an_adjacent_body_is_executed_before_the_brain_aims_or_fires_at_a_live_foe() {
    let mut app = brain_app();
    let pool = execute_cost(&app).saturating_add(ENGAGE_POOL);
    let actor = spawn_combatant(
        app.world_mut(),
        ground(5, 5),
        ENEMY,
        Direction::East,
        pool,
        6,
    );
    // Off the line east, so the body never stands between the actor and the live foe.
    let body_at = ground(6, 6);
    let live_at = ground(9, 5);
    let body = spawn_combatant(app.world_mut(), body_at, PLAYER, Direction::West, 100, 2);
    let live = spawn_combatant(app.world_mut(), live_at, PLAYER, Direction::West, 100, 2);
    place_occupant(&mut app, body_at, body);
    place_occupant(&mut app, live_at, live);
    down_before_the_enemy_turn(&mut app, &[body]);

    let drive = drive_until_player(&mut app);

    let executed_on = drive.first_execute(actor);
    let aimed_on = drive.first_aim(actor);
    let fired_on = drive.first_fire(actor);
    assert!(
        matches!(
            (executed_on, aimed_on, fired_on),
            (Some(execute), Some(aim), Some(fire)) if execute < aim && execute < fire
        ),
        "the downed arm runs ahead of aim and fire, so the adjacent body must be executed on \
         a strictly earlier frame than the first aim and the first shot at the live foe: \
         execute {executed_on:?}, aim {aimed_on:?}, fire {fired_on:?}",
    );
}

#[test]
fn spawn_order_never_decides_which_of_two_adjacent_bodies_is_executed() {
    for spawn_low_first in [true, false] {
        let scene = drive_two_adjacent_bodies(spawn_low_first);

        assert_eq!(
            scene.drive.executes_for(scene.actor, scene.low).len(),
            1,
            "pick_nearest orders equally adjacent rows by cell, so the body at (6,4) is the one \
             executed whichever row the brain reads first (low spawned first: {spawn_low_first}): \
             {:?}",
            scene.drive.executes,
        );
        assert!(
            scene.drive.executes_for(scene.actor, scene.high).is_empty(),
            "the pool covers one execute, and it must not go to the body at (6,6) \
             (low spawned first: {spawn_low_first}): {:?}",
            scene.drive.executes,
        );
    }
}
