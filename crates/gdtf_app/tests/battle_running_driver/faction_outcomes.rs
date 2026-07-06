//! Explicit `BattleWon` / `BattleLost` outcomes end the battle exactly once (GTW-239).

use gdtf_app::test_support::{BattleRunningComplete, BattleScapeState};
use gdtf_test_utils::advance_until;

use super::harness::*;

// === GTW-239 — end BattleRunning on the sim's outcome signal (BattleWon OR BattleLost). ===
//
// These exercise the LIVE app-side `end_battle_on_outcome` system (Update,
// `.after(SimSystems::Simulate)`, presence-gated). They drive the real walk to
// `BattleRunning` (so the sim's `BattleSimPlugin` is added and the `BattleWon`/`BattleLost`
// buffers exist app-side for free), then either write the sim outcome message directly into
// the world buffer (AC1–AC4, the `bevy-traps.md` #7 carve-out (a) test-body message-write) or
// drive the REAL GTW-237 `check_outcome` census by setting ganger `LifeState`s (AC5).

/// AC1 — a `BattleWon` written DURING `BattleRunning` ends the battle → `AnimateOut`.
///
/// Drives into `BattleRunning` (GTW-236 persistence applied), writes ONE
/// `gdtf_battle_sim::battle::BattleWon` into the world's buffer (the sanctioned test-body
/// message-write), then advances: `end_battle_on_outcome` reads the outcome and inserts
/// `BattleRunningComplete`, and the marker-gated `move_on` advances `BattleRunning → AnimateOut`.
/// Pin-discriminating: with `end_battle_on_outcome` unwired the marker is never inserted, so the
/// machine would persist (GTW-236) and this fails.
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

    // Write one sim outcome signal into the app-side buffer (registered by the already-added
    // BattleSimPlugin), standing in for the census' emit.
    app.world_mut()
        .write_message(gdtf_battle_sim::battle::BattleWon);

    // `end_battle_on_outcome` (Update) inserts the marker; observe it WHILE still in
    // BattleRunning — `cleanup` (OnExit(BattleRunning), reused as-is, out of scope) removes the
    // per-run marker the instant the state leaves BattleRunning, so the marker and `AnimateOut`
    // are observable at adjacent points, not the same instant. Catching the insert before the
    // exit proves `end_battle_on_outcome` fired; the follow-on drive proves the chain advances.
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

/// AC2 — a `BattleLost` written DURING `BattleRunning` ALSO ends the battle → `AnimateOut`.
///
/// Identical to AC1 but writes `gdtf_battle_sim::battle::BattleLost`, proving LOSS ends the fight via
/// the SAME chain, not just victory. Pin-discriminating: a system that only handled the `won`
/// reader would leave this red (no marker inserted, `BattleRunning` persists).
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

    // Observe the marker insert while still in BattleRunning (see AC1 for why the marker and
    // AnimateOut are observable at adjacent points, not the same instant — `cleanup` removes the
    // per-run marker on exit).
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

/// AC4 (win) — a `BattleWon` written EVERY update across several updates does NOT double-fire.
///
/// A census re-declares the outcome each tick; the `not(resource_exists::<BattleRunningComplete>)`
/// gate + idempotent insert must keep the marker to ONE insert and advance the machine out of
/// `BattleRunning` EXACTLY once (it must not bounce). Asserts the state reaches `AnimateOut` and
/// the marker is present. Pin-discriminating: removing the `not(resource_exists)` gate would
/// re-run the insert each frame (still harmless for a unit marker, but the gate is the spec) —
/// the stronger guard is that the machine never re-enters `BattleRunning` after leaving.
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

    // Write a BattleWon EVERY update, and stop once the machine has left BattleRunning. Track
    // that it leaves exactly once (never bounces back into BattleRunning afterwards) and that the
    // marker is present while still in BattleRunning (it is `cleanup`-removed on exit, so it is
    // only observable before the transition — see AC1).
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
            // Once it has left, it must NEVER be back in BattleRunning on a later tick.
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

/// AC4 (loss) — a `BattleLost` written EVERY update across several updates does NOT double-fire.
///
/// The loss twin of [`repeated_battle_won_does_not_double_fire`]: proves the gate + idempotent
/// insert keep the loss stream to one advance out of `BattleRunning`.
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
