use bevy::time::TimeUpdateStrategy;
use cobalt_test_utils::advance_until;
use gdtf_game::test_support::{
    AutoRunning, AutoStepDelay, BattleScapeState, PendingStepCommand, StepCommand,
};

use super::harness::{
    FIXED_SEED, app_engaged_in_generation, app_ready_for_battle, battlescape_state,
    drive_into_battle_running, terrain_fingerprint,
};

#[test]
fn skip_drives_every_remaining_stage_in_one_request() {
    let expected = {
        let mut app = app_ready_for_battle(FIXED_SEED);
        drive_into_battle_running(&mut app);
        terrain_fingerprint(&app)
    };

    let mut app = app_engaged_in_generation(FIXED_SEED);

    let pending = app.world_mut().get_resource_mut::<PendingStepCommand>();
    assert!(
        pending.is_some(),
        "PendingStepCommand must exist while the stepper is engaged in Generation",
    );
    if let Some(mut pending) = pending {
        pending.request(StepCommand::Skip);
    }
    app.update();

    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });

    let actual = terrain_fingerprint(&app);
    assert_eq!(
        actual, expected,
        "Skip's stepped-to-completion drive must produce the SAME terrain fingerprint as the \
         normal to-completion path for the same seed",
    );
}

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
    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);

    let actual = terrain_fingerprint(&app);
    assert_eq!(
        actual, expected,
        "Auto's free-run drive must produce the SAME terrain fingerprint as the normal \
         to-completion path for the same seed",
    );
}
