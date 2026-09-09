use gdtf_battle_sim::{
    acts::{EndTurnRequested, MoveRequested, movement::ReactionShotFired},
    ganger::Direction,
    test_support::SituationBuilder,
};

use super::harness::*;

#[test]
fn ac1_ac2_ac7_acting_player_is_interrupted_by_an_enemy_watcher() {
    let mut app = battle_app(forced_reaction_tuning(8));
    with_shot_log(&mut app);

    let watcher_cell = ground(4, 5);
    let player_start = ground(6, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            mover(player_start, PLAYER, Direction::East),
            watcher(watcher_cell, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(player), Some(enemy)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player and one enemy");
    };
    let Some(enemy_tu_before) = tu_of(&app, enemy) else {
        unreachable!("the enemy has a Tu pool");
    };
    let Some(player_start_now) = pos_of(&app, player) else {
        unreachable!("the player has a Position");
    };

    let dest = ground(10, 5);
    app.world_mut()
        .write_message(MoveRequested::new(player, dest));
    run_until_walk_ends(&mut app, player);
    step(&mut app, 3);

    let Some(player_final) = pos_of(&app, player) else {
        unreachable!("the player persists");
    };
    assert!(
        !is_walking(&app, player),
        "AC2: the interrupt removes the player's WalkInProgress (the walk halted)",
    );
    assert_ne!(
        player_final, dest,
        "AC2: the interrupt halts the walk SHORT of the destination",
    );
    assert_ne!(
        player_final, player_start_now,
        "the player took at least one step before the interrupt fired",
    );

    let Some(enemy_tu_after) = tu_of(&app, enemy) else {
        unreachable!("the enemy persists");
    };
    assert!(
        enemy_tu_after < enemy_tu_before,
        "AC1: the watcher's TU is debited by the interrupt shot ({enemy_tu_after} < \
         {enemy_tu_before})",
    );

    assert!(
        shots_by(&app, enemy) >= 1,
        "AC1: the enemy watcher fired at least one interrupt round (ShotFired from the \
         watcher) — the trigger is LIVE",
    );

    assert!(
        all_shots_have_reports(&app, enemy),
        "AC7: the reaction shot resolves through the NORMAL fire pipeline (every ShotFired \
         carries a HitReport — no reaction damage modifier / no bespoke path)",
    );

    assert!(
        used_of(&app, enemy).is_some_and(|u| u >= 1),
        "the watcher's ReactionsUsed incremented when it interrupted",
    );
}

#[test]
fn ac3_no_interrupt_when_the_actor_acts_outside_los() {
    let mut app = battle_app(forced_reaction_tuning(8));
    with_shot_log(&mut app);

    let watcher_cell = ground(40, 40);
    let player_start = ground(6, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            mover(player_start, PLAYER, Direction::East),
            watcher(watcher_cell, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(player), Some(enemy)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player and one enemy");
    };
    let Some(enemy_tu_before) = tu_of(&app, enemy) else {
        unreachable!("the enemy has a Tu pool");
    };

    let dest = ground(10, 5);
    app.world_mut()
        .write_message(MoveRequested::new(player, dest));
    run_until_walk_ends(&mut app, player);
    step(&mut app, 3);

    assert_eq!(
        shots_by(&app, enemy),
        0,
        "AC3: an out-of-range watcher fires NO interrupt (LOS/range gate held)",
    );
    assert_eq!(
        tu_of(&app, enemy),
        Some(enemy_tu_before),
        "AC3: the out-of-range watcher spent no TU",
    );
    assert_eq!(
        used_of(&app, enemy),
        Some(0),
        "AC3: the out-of-range watcher's cap counter never incremented",
    );
    assert_eq!(
        pos_of(&app, player),
        Some(dest),
        "AC3: with no interrupt the player walks freely to its destination",
    );
}

#[test]
fn ac5_a_player_watcher_interrupts_an_acting_enemy_on_the_enemy_turn() {
    let mut app = battle_app(forced_reaction_tuning(8));
    with_shot_log(&mut app);

    let player_watcher = ground(5, 5);
    let enemy_at = ground(15, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(player_watcher, PLAYER, Direction::East),
            mover(enemy_at, ENEMY, Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let Some(player_watcher_entity) = ganger_of(&mut app, PLAYER) else {
        unreachable!("setup spawns the player watcher");
    };
    let Some(enemy) = ganger_of(&mut app, ENEMY) else {
        unreachable!("setup spawns the enemy");
    };
    let Some(watcher_tu_before) = tu_of(&app, player_watcher_entity) else {
        unreachable!("the player watcher has a Tu pool");
    };

    app.world_mut().write_message(EndTurnRequested);
    let mut tu_at_interrupt = None;
    for _ in 0..16 {
        app.update();
        if shots_by(&app, player_watcher_entity) >= 1 {
            tu_at_interrupt = tu_of(&app, player_watcher_entity);
            break;
        }
    }

    assert!(
        shots_by(&app, player_watcher_entity) >= 1,
        "AC5: the PLAYER watcher fired an interrupt at the BRAIN-driven acting enemy during \
         the enemy turn (faction symmetry — the mirror of AC1)",
    );
    assert!(
        tu_at_interrupt.is_some_and(|tu| tu < watcher_tu_before),
        "AC5: the player watcher's TU is debited by its interrupt of the enemy",
    );
    assert!(
        all_shots_have_reports(&app, player_watcher_entity),
        "AC5/AC7: the player watcher's interrupt resolves through the normal fire pipeline",
    );
    assert!(
        pos_of(&app, enemy).is_some_and(|p| p != enemy_at),
        "AC5: the enemy ACTUALLY moved (the brain drove it) — the interrupt saw a real step",
    );
}

#[test]
fn the_interrupt_sequence_is_deterministic_across_identical_runs() {
    let run_once = || {
        let mut app = battle_app(forced_reaction_tuning(8));
        with_shot_log(&mut app);
        let situation = SituationBuilder::new()
            .with_gangers([
                mover(ground(6, 5), PLAYER, Direction::East),
                watcher(ground(4, 5), ENEMY, Direction::East),
            ])
            .build_with_gangs();
        drive_setup(&mut app, situation);
        let (Some(player), Some(enemy)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
        else {
            unreachable!("setup spawns one player and one enemy");
        };
        app.world_mut()
            .write_message(MoveRequested::new(player, ground(10, 5)));
        run_until_walk_ends(&mut app, player);
        step(&mut app, 3);
        (shots_by(&app, enemy), pos_of(&app, player))
    };

    let first = run_once();
    let second = run_once();
    assert_eq!(
        first, second,
        "the same seed + tuning reproduces the same interrupt outcome (replay-stable \
         ReactionRng through the live path)",
    );
    assert!(
        first.0 >= 1,
        "precondition: the deterministic run actually interrupts (forced p == 1.0)",
    );
    let _ = ReactionShotFired::new;
}
