//! The per-turn reaction cap: it bites within a turn and resets next turn (AC4).

use gdtf_battle_sim::{acts::MoveRequested, ganger::Direction, test_support::SituationBuilder};

use super::harness::*;

// === AC4 — the per-turn cap bites and resets next turn. With cap == 1, a watcher interrupts
// at most once this turn, then reacts AGAIN after a turn boundary resets the counter. ===

#[test]
fn ac4_the_per_turn_cap_bites_then_resets_next_turn() {
    // cap == 1: EXACTLY one interrupt per watcher PER TURN. The reactor is a PLAYER watcher;
    // the ACTOR is an ENEMY moved by explicit `MoveRequested`s (a non-player mover routes over
    // the never-stale OMNISCIENT move fog through `dispatch_move`, so its walk is reliable
    // turn after turn — and it only ever MOVES, never engages, so it can never kill the
    // watcher mid-test). Both gangers carry FAT HP pools so neither is downed by an interrupt
    // shot across the multi-turn run. The acting ganger is driven THROUGH the real move /
    // dispatch / advance_walk path (NOT a synthetic emit). Cap-bite: the watcher interrupts
    // ONCE this turn and not again. Reset: after a turn boundary the watcher reacts AGAIN.
    let mut app = battle_app(forced_reaction_tuning(1));
    with_shot_log(&mut app);

    // The player watcher at (5,5) facing East; a tough enemy a couple cells east at (7,5),
    // ALREADY in the watcher's LOS at spawn (so the enemy's walk never reveal-halts — only
    // the reaction interrupt halts it).
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

    // === Turn 1: move the enemy east in the watcher's LOS → the watcher interrupts ONCE. ===
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

    // A SECOND enemy move THIS SAME turn must NOT draw another interrupt (the cap bit).
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

    // === Cross a FULL turn cycle back to the PLAYER's turn. This (a) resets the watcher's
    // per-turn cap counter at each TurnStarted boundary AND (b) regenerates the player
    // watcher's TU at the player turn-start — so on its next turn the watcher both has cap
    // room AND can AFFORD the interrupt shot again (a watcher that spent its TU reacting
    // genuinely cannot react until its TU regenerates — the §8 TU economy). ===
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

    // === A fresh enemy act after the reset draws a FRESH interrupt (the cap re-opened and the
    // watcher's TU regenerated). Move the enemy through the watcher's LOS lane again. ===
    let shots_before_reset_act = shots_by(&app, watcher_entity);
    let Some(enemy_now) = pos_of(&app, enemy) else {
        unreachable!("the enemy persists across the turn cycle");
    };
    // Move the enemy EAST (away from the now-adjacent watcher) but staying inside its LOS
    // lane + range — a clean act-in-LOS step the watcher can interrupt. The brain may have
    // walked the enemy right up to the watcher during the cycle, so moving away guarantees an
    // actual step (a move onto its own cell would be a no-op). The destination stays within
    // the watcher's view range (Chebyshev ≤ 6 from the watcher cell).
    let away_x = (enemy_now.x + 3).min(player_watcher.x + 5);
    app.world_mut()
        .write_message(MoveRequested::new(enemy, ground(away_x, player_watcher.y)));
    run_until_walk_ends(&mut app, enemy);
    step(&mut app, 3);
    assert!(
        shots_by(&app, watcher_entity) > shots_before_reset_act,
        "AC4: after the reset the watcher reacts AGAIN next turn (the cap re-opened): \
         {shots_before_reset_act} → {}",
        shots_by(&app, watcher_entity),
    );
}
