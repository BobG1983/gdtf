use gdtf_app::test_support::{BattleRunningComplete, BattleScapeState};
use gdtf_test_utils::advance_until;

use super::harness::*;

// === GTW-239 — end BattleRunning on the sim's outcome signal (BattleWon OR BattleLost). ===

#[test]
fn battle_won_in_battle_running_ends_the_battle_to_animate_out() {
    let app_opt = driven_battle_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    app.world_mut()
        .write_message(gdtf_battle_sim::battle::BattleWon);

    let marker_inserted = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<BattleRunningComplete>()
                .is_some()
        },
        BUDGET,
    );
    assert!(
        marker_inserted,
        "end_battle_on_outcome must insert the BattleRunningComplete marker on BattleWon within \
         {BUDGET} updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    let reached_animate_out = advance_until(&mut app, left_battle_running, BUDGET);
    assert!(
        reached_animate_out,
        "a BattleWon in BattleRunning must end the battle (end_battle_on_outcome inserts the \
         marker, move_on advances) within {BUDGET} updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::AnimateOut),
        "a BattleWon must advance BattleRunning → AnimateOut (the same chain as the explicit end)",
    );
}

#[test]
fn battle_lost_in_battle_running_also_ends_the_battle_to_animate_out() {
    let app_opt = driven_battle_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    app.world_mut()
        .write_message(gdtf_battle_sim::battle::BattleLost);

    let marker_inserted = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<BattleRunningComplete>()
                .is_some()
        },
        BUDGET,
    );
    assert!(
        marker_inserted,
        "end_battle_on_outcome must insert the BattleRunningComplete marker on BattleLost within \
         {BUDGET} updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    let reached_animate_out = advance_until(&mut app, left_battle_running, BUDGET);
    assert!(
        reached_animate_out,
        "a BattleLost in BattleRunning must ALSO end the battle within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::AnimateOut),
        "a BattleLost must end the battle via the SAME BattleRunning → AnimateOut chain as a win",
    );
}

#[test]
fn repeated_battle_won_does_not_double_fire() {
    let app_opt = driven_battle_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    let mut left_once = false;
    let mut marker_seen_in_running = false;
    for _ in 0..BUDGET {
        app.world_mut()
            .write_message(gdtf_battle_sim::battle::BattleWon);
        app.update();
        if !left_battle_running(&app)
            && app
                .world()
                .get_resource::<BattleRunningComplete>()
                .is_some()
        {
            marker_seen_in_running = true;
        }
        if left_battle_running(&app) {
            left_once = true;
            assert_ne!(
                battlescape_state(&app),
                Some(BattleScapeState::BattleRunning),
                "the machine must not bounce back into BattleRunning after a repeated BattleWon",
            );
        }
    }
    assert!(
        left_once,
        "a repeated BattleWon must advance the machine out of BattleRunning within {BUDGET} \
         updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert!(
        marker_seen_in_running,
        "the BattleRunningComplete marker must have been inserted (observed present in \
         BattleRunning) under a repeated BattleWon",
    );
}

#[test]
fn repeated_battle_lost_does_not_double_fire() {
    let app_opt = driven_battle_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    let mut left_once = false;
    let mut marker_seen_in_running = false;
    for _ in 0..BUDGET {
        app.world_mut()
            .write_message(gdtf_battle_sim::battle::BattleLost);
        app.update();
        if !left_battle_running(&app)
            && app
                .world()
                .get_resource::<BattleRunningComplete>()
                .is_some()
        {
            marker_seen_in_running = true;
        }
        if left_battle_running(&app) {
            left_once = true;
            assert_ne!(
                battlescape_state(&app),
                Some(BattleScapeState::BattleRunning),
                "the machine must not bounce back into BattleRunning after a repeated BattleLost",
            );
        }
    }
    assert!(
        left_once,
        "a repeated BattleLost must advance the machine out of BattleRunning within {BUDGET} \
         updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert!(
        marker_seen_in_running,
        "the BattleRunningComplete marker must have been inserted (observed present in \
         BattleRunning) under a repeated BattleLost",
    );
}
