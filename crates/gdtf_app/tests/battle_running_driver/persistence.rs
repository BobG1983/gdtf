//! `BattleRunning` persists until an explicit end signal (GTW-236 lifecycle).

use gdtf_app::test_support::{BattleRunningComplete, BattleScapeState};
use gdtf_test_utils::advance_until;

use super::harness::*;

/// The persistence stress count — N ≫ 3 (the deleted placeholder budget was 3 ticks),
/// so a re-added auto-exit (which fired within ~3 `FixedUpdate` ticks) would advance OFF
/// `BattleRunning` well within this loop and turn the persistence assertion red.
const PERSIST_UPDATES: u32 = 32;

/// AC1/AC2 — PERSISTENCE: with no end-signal marker inserted, `BattleRunning` PERSISTS
/// across N ≫ 3 (`PERSIST_UPDATES`) updates — it never reaches `AnimateOut`/`AfterMath`.
///
/// This is the core GTW-236 change: the placeholder 3-tick turn-budget auto-exit is gone, so
/// no budget/timer/tick system inserts `BattleRunningComplete` on its own. A regression that
/// re-adds an auto-insert would trip `move_on` (advancing to `AnimateOut`) within ~3 ticks,
/// failing the per-iteration assert.
#[test]
fn battle_running_persists_without_an_end_signal() {
    let app_opt = driven_battle_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    // Across many updates with NO explicit marker, the machine stays put in BattleRunning.
    for iteration in 0..PERSIST_UPDATES {
        app.update();
        assert_eq!(
            battlescape_state(&app),
            Some(BattleScapeState::BattleRunning),
            "BattleRunning must PERSIST with no BattleRunningComplete inserted (GTW-236) — it must \
             not auto-advance to AnimateOut; failed on update {iteration} of {PERSIST_UPDATES}",
        );
    }
}

/// AC3 — the explicit end still works: while resting in `BattleRunning`, inserting
/// `BattleRunningComplete` (through the `test_support` surface, standing in for the
/// not-yet-wired victory/flee end condition) then `update()`-ing advances `BattleScapeState`
/// to `AnimateOut` — proving the marker-gated `move_on` registration survived the rework.
#[test]
fn explicit_end_marker_advances_to_animate_out() {
    let app_opt = driven_battle_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    // Insert the explicit end-signal marker (the one victory or flee will drive).
    app.world_mut().insert_resource(BattleRunningComplete);

    let reached_animate_out = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::AnimateOut),
        BUDGET,
    );
    assert!(
        reached_animate_out,
        "an explicit BattleRunningComplete insert must trip move_on and advance BattleRunning → \
         AnimateOut within {BUDGET} updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
}

/// AC3 — with NO outcome signal, `BattleRunning` PERSISTS (the new system is inert).
///
/// Drives into `BattleRunning`, writes NEITHER outcome message, advances several updates, and
/// asserts `BattleRunningComplete` is ABSENT and the state is STILL `BattleRunning` — proving
/// the now-WIRED `end_battle_on_outcome` adds no spurious exit (re-asserts GTW-236 persistence
/// with the new system registered-but-quiescent). This differs from the GTW-236 persistence
/// test only in intent: that one proves no budget auto-exit; THIS one proves the GTW-239 system,
/// once in the schedule, stays quiet without an outcome.
#[test]
fn battle_running_persists_with_outcome_system_wired_but_quiescent() {
    let app_opt = driven_battle_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    for iteration in 0..PERSIST_UPDATES {
        app.update();
        assert!(
            app.world()
                .get_resource::<BattleRunningComplete>()
                .is_none(),
            "with NO outcome signal, end_battle_on_outcome must insert NO BattleRunningComplete \
             marker; failed on update {iteration} of {PERSIST_UPDATES}",
        );
        assert_eq!(
            battlescape_state(&app),
            Some(BattleScapeState::BattleRunning),
            "BattleRunning must PERSIST with end_battle_on_outcome wired-but-quiescent (no \
             outcome); failed on update {iteration} of {PERSIST_UPDATES}",
        );
    }
}
