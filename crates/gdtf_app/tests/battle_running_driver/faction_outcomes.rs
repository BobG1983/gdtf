use gdtf_app::test_support::{BattleRunningComplete, BattleScapeState};
use gdtf_test_utils::advance_until;

use super::harness::*;

/// Frames the machine is given to prove it does not bounce — per-frame work, no IO.
const HOLD_FRAMES: u32 = 32;

// === end BattleRunning on the sim's outcome signal (BattleWon OR BattleLost). ===

#[test]
fn battle_won_in_battle_running_ends_the_battle_to_animate_out() {
    let mut app = driven_battle_app();

    app.world_mut()
        .write_message(gdtf_battle_sim::battle::BattleWon);

    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<BattleRunningComplete>()
            .is_some()
    });

    advance_until(&mut app, left_battle_running);
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::AnimateOut),
        "a BattleWon must advance BattleRunning → AnimateOut (the same chain as the explicit end)",
    );
}

#[test]
fn battle_lost_in_battle_running_also_ends_the_battle_to_animate_out() {
    let mut app = driven_battle_app();

    app.world_mut()
        .write_message(gdtf_battle_sim::battle::BattleLost);

    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<BattleRunningComplete>()
            .is_some()
    });

    advance_until(&mut app, left_battle_running);
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::AnimateOut),
        "a BattleLost must end the battle via the SAME BattleRunning → AnimateOut chain as a win",
    );
}

#[test]
fn repeated_battle_won_does_not_double_fire() {
    let mut app = driven_battle_app();

    let mut marker_seen_in_running = false;
    loop {
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
            break;
        }
    }
    for _ in 0..HOLD_FRAMES {
        app.world_mut()
            .write_message(gdtf_battle_sim::battle::BattleWon);
        app.update();
        assert_ne!(
            battlescape_state(&app),
            Some(BattleScapeState::BattleRunning),
            "the machine must not bounce back into BattleRunning after a repeated BattleWon",
        );
    }
    assert!(
        marker_seen_in_running,
        "the BattleRunningComplete marker must have been inserted (observed present in \
         BattleRunning) under a repeated BattleWon",
    );
}

#[test]
fn repeated_battle_lost_does_not_double_fire() {
    let mut app = driven_battle_app();

    let mut marker_seen_in_running = false;
    loop {
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
            break;
        }
    }
    for _ in 0..HOLD_FRAMES {
        app.world_mut()
            .write_message(gdtf_battle_sim::battle::BattleLost);
        app.update();
        assert_ne!(
            battlescape_state(&app),
            Some(BattleScapeState::BattleRunning),
            "the machine must not bounce back into BattleRunning after a repeated BattleLost",
        );
    }
    assert!(
        marker_seen_in_running,
        "the BattleRunningComplete marker must have been inserted (observed present in \
         BattleRunning) under a repeated BattleLost",
    );
}
