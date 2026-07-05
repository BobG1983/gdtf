//! C7(a) / C7(c) / C7(d) — what stops a committed walk: a bump into a newly occupied cell,
//! a newly revealed enemy, and a synthetic reaction interrupt.

use gdtf_battle_sim::{
    Faction, ReactionShotFired, Speed, SquadVisibility, Stance, StanceKind,
    acts::MoveRequested,
    occupancy::OccupancyGrid,
    test_support::{GangerSpawnBuilder, SituationBuilder},
};

use super::harness::*;

// === C7(a) — a walk whose next cell is OCCUPIED after planning BUMP-STOPS at the last
// free step, charged only the steps taken. ===

#[test]
fn walk_bump_stops_when_next_cell_becomes_occupied() {
    let mut app = battle_app();
    // Two players so we have a second body to drop into the route mid-walk; both gang 0
    // so neither blocks the other's route by visibility (own-squad), but an OCCUPANT slot
    // always bump-stops regardless of faction.
    let situation = SituationBuilder::new()
        .with_gangers([
            GangerSpawnBuilder::new()
                .at(player_at())
                .faction(Faction::new(PLAYER))
                .stance(Stance::new(StanceKind::Standing))
                .speed(Speed::new(20.0))
                .build(),
            GangerSpawnBuilder::new()
                .at(ground(20, 20))
                .faction(Faction::new(PLAYER))
                .stance(Stance::new(StanceKind::Standing))
                .speed(Speed::new(20.0))
                .build(),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let Some(actor) = player_entity(&mut app) else {
        unreachable!("setup spawns the player ganger");
    };
    let Some((start, tu_before)) = pos_and_tu(&app, actor) else {
        unreachable!("the player has a Position and Tu");
    };

    // A 3-cell east walk: (5,5) -> (6,5) -> (7,5) -> (8,5).
    let dest = ground(8, 5);
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    // One tick: the accept starts the walk and takes the FIRST step to (6,5).
    app.update();
    let Some((after_first, tu_after_first)) = pos_and_tu(&app, actor) else {
        unreachable!("the player persists");
    };
    assert_ne!(
        after_first, start,
        "precondition: the walk took its first step",
    );
    assert!(
        is_walking(&app, actor),
        "precondition: the walk is still in flight (more cells ahead)",
    );

    // Now drop an OBSTACLE onto the NEXT cell ahead (7,5) — occupied AFTER the route was
    // planned (occupancy_sync re-runs each tick, so a fresh occupant appears mid-walk).
    let blocker_cell = ground(7, 5);
    let blocker = app.world_mut().spawn_empty().id();
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(blocker_cell, Some(blocker));
    }

    // Run the walk to its end: it must BUMP-STOP before entering (7,5).
    run_until_walk_ends(&mut app, actor);

    let Some((final_cell, tu_final)) = pos_and_tu(&app, actor) else {
        unreachable!("the player persists");
    };
    assert_ne!(
        final_cell, blocker_cell,
        "the mover NEVER enters the now-occupied cell (no co-location)",
    );
    assert_ne!(
        final_cell, dest,
        "the bump-stop halts the walk SHORT of the destination",
    );
    // Charged only the steps actually taken: a strictly positive spend (it moved) that is
    // STRICTLY LESS than the full planned route would have cost (it stopped short).
    let Some(full_total) = plan_total(&app, start, dest) else {
        unreachable!("the full route was plannable at start");
    };
    let spent = tu_before.saturating_sub(tu_final);
    assert!(
        spent > 0 && spent < full_total,
        "a bump-stopped walk charges only the ground covered (steps taken), not the full \
         route: spent={spent}, full={full_total}",
    );
    // And it did move at least one cell (the first step landed before the obstacle).
    assert!(
        tu_after_first < tu_before,
        "the first step charged its cost",
    );
}

// === C7(c) — a walk that REVEALS a previously-UNSEEN enemy halts immediately, charged
// only the steps taken. ===

#[test]
fn walk_stops_when_a_new_enemy_is_revealed() {
    let mut app = battle_app();
    // An enemy parked east at (11,5), beyond TEST_VIEW_RANGE from the player spawn
    // (Chebyshev 6 > 4), so it is UNSEEN at walk start.
    let enemy_cell = ground(11, 5);
    drive_setup(&mut app, player_and_enemy_situation(20.0, enemy_cell));

    let Some(actor) = player_entity(&mut app) else {
        unreachable!("setup spawns the player ganger");
    };
    let Some((start, tu_before)) = pos_and_tu(&app, actor) else {
        unreachable!("the player has a Position and Tu");
    };

    // Walk east to (9,5) — within view range at spawn (Chebyshev 4, so routable) and short
    // of the enemy, but the approach brings the enemy into view (Chebyshev to (11,5)
    // shrinks below TEST_VIEW_RANGE mid-walk), revealing it.
    let dest = ground(9, 5);
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    run_until_walk_ends(&mut app, actor);

    let Some((final_cell, tu_final)) = pos_and_tu(&app, actor) else {
        unreachable!("the player persists");
    };
    // The reveal halts the walk SHORT of the destination (it stops the tick the enemy
    // enters the squad VISIBLE set).
    assert_ne!(
        final_cell, dest,
        "a revealed enemy halts the walk before the destination (the §44 ambush)",
    );
    assert_ne!(
        final_cell, start,
        "the walk took at least one step before the reveal",
    );
    // Charged only the steps taken (a positive, partial spend).
    let Some(full_total) = plan_total(&app, start, dest) else {
        unreachable!("the route to dest was plannable at start");
    };
    let spent = tu_before.saturating_sub(tu_final);
    assert!(
        spent > 0 && spent < full_total,
        "a reveal-stopped walk charges only the ground covered: spent={spent}, full={full_total}",
    );
    // The walk is no longer in flight (it was halted, not merely paused).
    assert!(
        !is_walking(&app, actor),
        "a revealed enemy removes the WalkInProgress (the walk halted)",
    );
    // POSITIVE: the enemy IS now in the squad VISIBLE set — the reveal mechanic (the
    // recompute the step's Position write triggered) is what halted the walk, not a
    // coincidental obstacle (the open route had none short of the enemy).
    let enemy_now_visible = app
        .world()
        .get_resource::<SquadVisibility>()
        .is_some_and(|squad| squad.is_cell_visible(&enemy_cell));
    assert!(
        enemy_now_visible,
        "the enemy entered the squad VISIBLE set — the reveal is what stopped the walk",
    );
}

// === C7(d) — a SYNTHETIC reaction-shot interrupt mid-walk halts the walk, charged only
// the steps taken. (The PRODUCER is GTW-38-future.) ===

#[test]
fn walk_stops_on_a_synthetic_reaction_interrupt() {
    let mut app = battle_app();
    drive_setup(&mut app, one_player_situation(20.0));

    let Some(actor) = player_entity(&mut app) else {
        unreachable!("setup spawns the player ganger");
    };
    let Some((start, tu_before)) = pos_and_tu(&app, actor) else {
        unreachable!("the player has a Position and Tu");
    };

    // Start a multi-step walk east.
    let dest = ground(9, 5);
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    // One tick: the walk starts and takes its first step.
    app.update();
    let Some((after_first, _)) = pos_and_tu(&app, actor) else {
        unreachable!("the player persists");
    };
    assert_ne!(
        after_first, start,
        "precondition: the walk took a first step"
    );
    assert!(
        is_walking(&app, actor),
        "precondition: the walk is still in flight",
    );

    // SYNTHETIC emit of the reaction-shot interrupt aimed at the mover (the GTW-38-future
    // producer would emit this; here the test plays the producer's role).
    app.world_mut().write_message(ReactionShotFired::new(actor));
    // The next advance_walk tick reads the interrupt and halts the walk.
    app.update();

    let Some((final_cell, tu_final)) = pos_and_tu(&app, actor) else {
        unreachable!("the player persists");
    };
    assert!(
        !is_walking(&app, actor),
        "a reaction-shot interrupt halts the walk (removes the WalkInProgress)",
    );
    assert_ne!(
        final_cell, dest,
        "the interrupt halts the walk before the destination",
    );
    // Charged only the steps taken before the interrupt (a positive, partial spend; the
    // interrupt itself charges nothing).
    let Some(full_total) = plan_total(&app, start, dest) else {
        unreachable!("the route to dest was plannable at start");
    };
    let spent = tu_before.saturating_sub(tu_final);
    assert!(
        spent > 0 && spent < full_total,
        "an interrupt-stopped walk charges only the ground covered: spent={spent}, \
         full={full_total}",
    );
}
