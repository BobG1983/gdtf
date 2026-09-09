use gdtf_battle_sim::{acts::MoveRequested, ganger::Direction, test_support::SituationBuilder};

use super::{harness::*, support::set_tu};

// === AC4 — the per-turn cap bites and resets next turn. With cap == 1, a watcher interrupts
// at most once this turn, then reacts AGAIN after a turn boundary resets the counter. ===

#[test]
fn ac4_the_per_turn_cap_bites_then_resets_next_turn() {
    let mut app = battle_app(forced_reaction_tuning(1));
    with_shot_log(&mut app);

    let player_watcher = ground(5, 5);
    let enemy_start = ground(7, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(player_watcher, PLAYER, Direction::East),
            tough_mover(enemy_start, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(watcher_entity), Some(enemy)) =
        (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player watcher and one enemy");
    };

    app.world_mut()
        .write_message(MoveRequested::new(enemy, ground(11, 5)));
    run_until_walk_ends(&mut app, enemy);
    step(&mut app, 3);
    let shots_first = shots_by(&app, watcher_entity);
    assert!(
        shots_first >= 1,
        "AC4: the watcher interrupted the enemy's move this turn (cap == 1, forced p == 1.0)",
    );
    assert_eq!(
        used_of(&app, watcher_entity),
        Some(1),
        "AC4: the watcher's per-turn cap counter is saturated at 1 this turn",
    );

    let shots_before_second = shots_by(&app, watcher_entity);
    let Some(enemy_after_first) = pos_of(&app, enemy) else {
        unreachable!("the enemy persists");
    };
    app.world_mut().write_message(MoveRequested::new(
        enemy,
        ground(enemy_after_first.x + 2, enemy_after_first.y),
    ));
    run_until_walk_ends(&mut app, enemy);
    step(&mut app, 3);
    assert_eq!(
        shots_by(&app, watcher_entity),
        shots_before_second,
        "AC4: the cap BITES — a second act this turn draws NO further interrupt",
    );

    cycle_back_to_player_turn(&mut app);
    assert!(
        player_turn_active(&app),
        "AC4 precondition: the turn cycled back to the player",
    );
    assert_eq!(
        used_of(&app, watcher_entity),
        Some(0),
        "AC4: the turn boundary RESET the watcher's per-turn cap counter",
    );

    let shots_before_reset_act = shots_by(&app, watcher_entity);
    let Some(enemy_now) = pos_of(&app, enemy) else {
        unreachable!("the enemy persists across the turn cycle");
    };
    set_tu(&mut app, enemy, 20);
    let dest = {
        let preferred = ground(player_watcher.x + 2, player_watcher.y + 1);
        if preferred == enemy_now {
            ground(player_watcher.x + 2, player_watcher.y - 1)
        } else {
            preferred
        }
    };
    app.world_mut()
        .write_message(MoveRequested::new(enemy, dest));
    run_until_walk_ends(&mut app, enemy);
    step(&mut app, 3);
    assert!(
        shots_by(&app, watcher_entity) > shots_before_reset_act,
        "AC4: after the reset the watcher reacts AGAIN next turn (the cap re-opened): \
         {shots_before_reset_act} → {}",
        shots_by(&app, watcher_entity),
    );
}
