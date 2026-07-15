//! The Skip / Auto app-wiring tests: each drives the stepper through the SAME latch resource the
//! egui panel's own button writes to, then asserts the drive free-runs to completion with no
//! further manual intervention.

use bevy::time::TimeUpdateStrategy;
use gdtf_app::test_support::{
    AutoRunning, AutoStepDelay, BattleScapeState, PendingStepCommand, StepCommand,
};
use gdtf_test_utils::advance_until;

use super::harness::{
    BUDGET, FIXED_SEED, app_engaged_in_generation, app_ready_for_battle, battlescape_state,
    drive_into_battle_running, terrain_fingerprint,
};

/// Skip, at the APP-WIRING level (C5a/C2): ONE [`StepCommand::Skip`] request — through the SAME
/// [`PendingStepCommand`] latch the panel's Skip button writes to, and the SAME
/// `advance_stepper_drive`/`finish_stepper_drive` systems the normal Next path runs through —
/// must drive every remaining stage to completion, not just one. Discriminating: a Skip that
/// only advanced a single stage (a Next-shaped bug) would leave the drive not-done with no
/// further command pending, stranding the app in `Generation` for the rest of the budget, so
/// `reached_running` below would be `false`.
#[test]
fn skip_drives_every_remaining_stage_in_one_request() {
    let expected = {
        let mut app = app_ready_for_battle(FIXED_SEED);
        drive_into_battle_running(&mut app);
        terrain_fingerprint(&app)
    };

    let mut app = app_engaged_in_generation(FIXED_SEED);

    // ONE Skip press — never touched again for the rest of this test.
    let pending = app.world_mut().get_resource_mut::<PendingStepCommand>();
    assert!(
        pending.is_some(),
        "PendingStepCommand must exist while the stepper is engaged in Generation",
    );
    if let Some(mut pending) = pending {
        pending.request(StepCommand::Skip);
    }
    app.update();

    let reached_running = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        reached_running,
        "a single Skip request must drive the WHOLE remaining staged pipeline to completion and \
         reach BattleRunning; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    let actual = terrain_fingerprint(&app);
    assert_eq!(
        actual, expected,
        "Skip's stepped-to-completion drive must produce the SAME terrain fingerprint as the \
         normal to-completion path for the same seed",
    );
}

/// Auto free-run, at the APP-WIRING level (C2): toggling [`AutoRunning::set`]`(true)` exactly
/// ONCE — the SAME latch the panel's "Start Auto" button writes to — and then only advancing the
/// app's virtual clock (never touching `PendingStepCommand` again) must free-run every remaining
/// stage on its own timer and reach `BattleRunning`. The clock is stepped in
/// [`AutoStepDelay::DEFAULT`]-sized increments via [`TimeUpdateStrategy::ManualDuration`] — the
/// SAME per-stage pace `AutoStepTimer` repeats on — so each `advance_until` update fires the
/// timer exactly once. Discriminating: if Auto never advanced the driver (a dead toggle), or
/// advanced only once, the app would strand in `Generation` and `reached_running` would be
/// `false`.
#[test]
fn auto_run_advances_every_stage_without_a_manual_command() {
    let expected = {
        let mut app = app_ready_for_battle(FIXED_SEED);
        drive_into_battle_running(&mut app);
        terrain_fingerprint(&app)
    };

    let mut app = app_engaged_in_generation(FIXED_SEED);

    let auto_running = app.world_mut().get_resource_mut::<AutoRunning>();
    assert!(
        auto_running.is_some(),
        "AutoRunning must exist while the stepper is engaged in Generation",
    );
    if let Some(mut auto_running) = auto_running {
        auto_running.set(true);
    }

    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            AutoStepDelay::DEFAULT.duration(),
        ));
    let reached_running = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
    assert!(
        reached_running,
        "toggling Auto on ONCE, with no further PendingStepCommand ever requested, must free-run \
         the whole staged drive to BattleRunning on its own timer; last observed \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    let actual = terrain_fingerprint(&app);
    assert_eq!(
        actual, expected,
        "Auto's free-run drive must produce the SAME terrain fingerprint as the normal \
         to-completion path for the same seed",
    );
}
