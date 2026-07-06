//! The full state walk: reaches Teardown, Teardown emits `AppExit`, deep back-pop quits.

use bevy::{
    app::{App, AppExit},
    state::state::State,
};
use gdtf_app::test_support::{
    AfterMathState, AppState, BattleRunningComplete, BattleScapeState, RunningState, app_state,
};
use gdtf_test_utils::advance_until;

use super::harness::*;

/// Drives the default-start walk all the way down to [`AppState::Teardown`] and returns
/// the app resting there, asserting each leg along the way (so a regression in the
/// transition graph fails in this shared driver, not just in one caller).
///
/// Standing in for the (not-yet-wired) player and victory/flee end conditions: it drives
/// past the menu (GTW-121), down to `BattleRunning` (GTW-236), verifies the battle
/// PERSISTS with no end-signal marker, then inserts the explicit `BattleRunningComplete`
/// marker which lets the deep terminal pop out to `Teardown`.
fn drive_to_teardown() -> App {
    let mut app = walk_app_with_theme();

    // The menu no longer auto-advances (GTW-121); stand in for the player to keep
    // the walk moving past it.
    assert!(
        drive_past_menu(&mut app),
        "the walk should reach RunningState::Menu within {WALK_BUDGET} updates; last observed \
         RunningState was {:?}",
        running_state(&app),
    );

    // The default walk now RESTS at BattleRunning (persistence, GTW-236).
    let reached_battle_running = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        WALK_BUDGET,
    );
    assert!(
        reached_battle_running,
        "the walk should descend to BattleScapeState::BattleRunning within {WALK_BUDGET} updates; \
         last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // With no end-signal marker, the machine must NOT auto-advance off BattleRunning:
    // the 3-tick auto-exit is gone, so the battle persists across the whole budget.
    for _ in 0..WALK_BUDGET {
        app.update();
        assert_eq!(
            battlescape_state(&app),
            Some(BattleScapeState::BattleRunning),
            "BattleRunning must PERSIST with no BattleRunningComplete inserted (GTW-236); the \
             placeholder turn-budget auto-exit must not advance it",
        );
        assert_ne!(
            app_state(&app),
            AppState::Teardown,
            "the walk must NOT reach Teardown on its own — it rests at BattleRunning until an \
             explicit end signal",
        );
    }

    // Insert the explicit end-signal marker (standing in for victory/flee), then the
    // marker-gated `move_on` advances BattleRunning → AnimateOut → … and the deep
    // terminal ultimately pops out to Teardown.
    app.world_mut().insert_resource(BattleRunningComplete);
    let reached = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Teardown,
        WALK_BUDGET,
    );
    assert!(
        reached,
        "an explicit BattleRunningComplete insert should advance the walk to AppState::Teardown \
         within {WALK_BUDGET} updates; last observed AppState was {:?}",
        app_state(&app),
    );

    app
}

/// (a) From the default start, the deep walk RESTS at
/// [`BattleScapeState::BattleRunning`] (it does NOT reach [`AppState::Teardown`] on
/// the walk alone, GTW-236) and reaches `Teardown` ONLY after an explicit
/// [`BattleRunningComplete`] insert stands in for the not-yet-wired victory/flee end
/// condition.
///
/// Pin: this fails if any top-level transition target regresses (so the walk stalls
/// before `BattleRunning`); it ALSO fails if the placeholder auto-exit ever returns
/// (the walk would reach `Teardown` on its own, before the explicit insert, and the
/// rest-at-`BattleRunning` assertion would catch the early advance). After the
/// insert, a regression in the deep terminal popping back out to `Teardown` keeps
/// the predicate unmet within budget.
#[test]
fn full_walk_reaches_teardown() {
    // The shared driver asserts the whole walk down to Teardown; reaching it without a
    // panic IS the assertion for this test.
    let app = drive_to_teardown();
    assert_eq!(
        app_state(&app),
        AppState::Teardown,
        "the driven walk should rest in AppState::Teardown",
    );
}

/// (a′) GTW-311: once the walk reaches [`AppState::Teardown`], the terminal `move_on`
/// system emits [`AppExit::Success`] within a bounded number of updates.
///
/// Pin: this is the regression that let the macOS shutdown hang (Bevy issue #23313)
/// ship — the prior `state_walk` reached `Teardown` but never asserted the exit fired,
/// so a `move_on` that silently failed to quit was invisible. This proves the HEADLESS
/// exit path (`exit.write(AppExit::Success)`) still fires: `App::should_exit()` reads
/// the `Messages<AppExit>` buffer, so checking it after each update catches the message
/// regardless of the message double-buffer's clear timing. The macOS window-despawn path
/// (winit consuming the close to terminate the loop) is NOT headless-testable — there is
/// no window under `MinimalPlugins` — and is verified by the in-engine playtest
/// (verification.md rule 3).
#[test]
fn teardown_emits_app_exit() {
    let mut app = drive_to_teardown();

    // After Teardown is entered, `teardown_complete` inserts the marker (1-tick deferred
    // Commands handoff) and then the marker-gated `move_on` writes `AppExit::Success`.
    // Poll `should_exit` each update so the assertion does not depend on the message
    // buffer's clear timing.
    let mut observed_exit = None;
    for _ in 0..WALK_BUDGET {
        app.update();
        if let Some(exit) = app.should_exit() {
            observed_exit = Some(exit);
            break;
        }
    }

    assert_eq!(
        observed_exit,
        Some(AppExit::Success),
        "Teardown's move_on must emit AppExit::Success within {WALK_BUDGET} updates of reaching \
         Teardown (GTW-311); observed {observed_exit:?}",
    );
}

/// (d) The deep terminal pops out correctly: the `AfterMath` sub-machine
/// finishes into [`RunningState::Quit`], and only then does [`AppState`] reach
/// [`Teardown`] — the machine does not wrap back to an earlier state.
///
/// Pin: this asserts the exact deep-pop order. First it confirms the deepest
/// sub-state [`AfterMathState::AnimateOut`] is actually visited (proving the
/// walk descends the whole battlescape/aftermath chain rather than short-
/// circuiting). Then it confirms `RunningState` reaches `Quit` (the terminal
/// pop) and that `AppState` reaches `Teardown`. If the aftermath terminal
/// regressed to re-enter the game (a wrap/loop) instead of popping to `Quit`,
/// `RunningState::Quit` would never be observed and the assertion fails.
#[test]
fn deep_pop_to_quit() {
    let mut app = walk_app_with_theme();

    // The menu no longer auto-advances (GTW-121); stand in for the player.
    assert!(
        drive_past_menu(&mut app),
        "the walk should reach RunningState::Menu within {WALK_BUDGET} updates; last observed \
         RunningState was {:?}",
        running_state(&app),
    );

    // The battlescape now persists in BattleRunning (GTW-236) — drive down to it,
    // then insert the explicit end-signal marker (standing in for victory/flee) so
    // the marker-gated `move_on` advances the chain past BattleRunning.
    let reached_battle_running = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        WALK_BUDGET,
    );
    assert!(
        reached_battle_running,
        "the walk should descend to BattleScapeState::BattleRunning within {WALK_BUDGET} updates; \
         last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    app.world_mut().insert_resource(BattleRunningComplete);

    // Prove the walk descends all the way into the deepest aftermath phase.
    let reached_aftermath_out = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<State<BattleScapeState>>()
                .is_some_and(|state| *state.get() == BattleScapeState::AfterMath)
                && app
                    .world()
                    .get_resource::<State<AfterMathState>>()
                    .is_some_and(|state| *state.get() == AfterMathState::AnimateOut)
        },
        WALK_BUDGET,
    );
    assert!(
        reached_aftermath_out,
        "the walk should descend into BattleScapeState::AfterMath / AfterMathState::AnimateOut \
         within {WALK_BUDGET} updates",
    );

    // The aftermath terminal pops out to RunningState::Quit (not back into Game).
    let reached_quit = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Quit),
        WALK_BUDGET,
    );
    assert!(
        reached_quit,
        "the AfterMath terminal should pop all the way out to RunningState::Quit, not wrap back \
         into the game; last observed RunningState was {:?}",
        running_state(&app),
    );

    // And Quit advances the top-level machine to Teardown — the terminal end.
    let reached_teardown = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Teardown,
        WALK_BUDGET,
    );
    assert!(
        reached_teardown,
        "RunningState::Quit should advance AppState to Teardown; last observed AppState was {:?}",
        app_state(&app),
    );
}
