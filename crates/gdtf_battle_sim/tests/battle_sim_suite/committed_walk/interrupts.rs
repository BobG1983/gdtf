use gdtf_battle_sim::{
    acts::{MoveRequested, movement::ReactionShotFired},
    ganger::Speed,
    occupancy::OccupancyGrid,
    prelude::{Faction, Stance, StanceKind},
    test_support::{GangerSpawnBuilder, SituationBuilder},
    visibility::SquadVisibility,
};

use super::harness::*;

#[test]
fn walk_bump_stops_when_next_cell_becomes_occupied() {
    let mut app = battle_app();
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

    let dest = ground(8, 5);
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
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

    let blocker_cell = ground(7, 5);
    let blocker = app.world_mut().spawn_empty().id();
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(blocker_cell, Some(blocker));
    }

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
    let Some(full_total) = plan_total(&app, start, dest) else {
        unreachable!("the full route was plannable at start");
    };
    let spent = tu_before.saturating_sub(tu_final);
    assert!(
        spent > 0 && spent < full_total,
        "a bump-stopped walk charges only the ground covered (steps taken), not the full \
         route: spent={spent}, full={full_total}",
    );
    assert!(
        tu_after_first < tu_before,
        "the first step charged its cost",
    );
}

#[test]
fn walk_stops_when_a_new_enemy_is_revealed() {
    let mut app = battle_app();
    let enemy_cell = ground(11, 5);
    drive_setup(&mut app, player_and_enemy_situation(20.0, enemy_cell));

    let Some(actor) = player_entity(&mut app) else {
        unreachable!("setup spawns the player ganger");
    };
    let Some((start, tu_before)) = pos_and_tu(&app, actor) else {
        unreachable!("the player has a Position and Tu");
    };

    let dest = ground(9, 5);
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    run_until_walk_ends(&mut app, actor);

    let Some((final_cell, tu_final)) = pos_and_tu(&app, actor) else {
        unreachable!("the player persists");
    };
    assert_ne!(
        final_cell, dest,
        "a revealed enemy halts the walk before the destination (the §44 ambush)",
    );
    assert_ne!(
        final_cell, start,
        "the walk took at least one step before the reveal",
    );
    let Some(full_total) = plan_total(&app, start, dest) else {
        unreachable!("the route to dest was plannable at start");
    };
    let spent = tu_before.saturating_sub(tu_final);
    assert!(
        spent > 0 && spent < full_total,
        "a reveal-stopped walk charges only the ground covered: spent={spent}, full={full_total}",
    );
    assert!(
        !is_walking(&app, actor),
        "a revealed enemy removes the WalkInProgress (the walk halted)",
    );
    let enemy_now_visible = app
        .world()
        .get_resource::<SquadVisibility>()
        .is_some_and(|squad| *squad.is_cell_visible(&enemy_cell));
    assert!(
        enemy_now_visible,
        "the enemy entered the squad VISIBLE set — the reveal is what stopped the walk",
    );
}

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

    let dest = ground(9, 5);
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
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

    app.world_mut().write_message(ReactionShotFired::new(actor));
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
