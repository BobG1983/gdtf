use bevy::{app::App, ecs::message::Messages};
use gdtf_battle_sim::{
    battle::SetupBattleRequested,
    procgen::{ProcgenAdvance, StagedProcgen},
};
use gdtf_game::test_support::{
    BattleGenerationContext, BattleScapeState, PendingStepCommand, StepCommand,
};

use super::harness::{FIXED_SEED, app_engaged_in_generation, battlescape_state};

/// Frames the finished generation is given to prove it writes nothing more.
const FRAMES_AFTER_FINISH: u32 = 4;

// How far the driver has got, or None once the finish has cleared it away.
fn placements(app: &App) -> Option<usize> {
    app.world()
        .get_resource::<StagedProcgen>()
        .map(|driver| driver.placed_footprints().len())
}

fn requests(app: &App) -> usize {
    app.world()
        .get_resource::<Messages<SetupBattleRequested>>()
        .map_or(0, Messages::len)
}

fn has_context(app: &App) -> bool {
    app.world()
        .get_resource::<BattleGenerationContext>()
        .is_some()
}

fn drain_requests(app: &mut App) {
    let _drained: Vec<SetupBattleRequested> = app
        .world_mut()
        .resource_mut::<Messages<SetupBattleRequested>>()
        .drain()
        .collect();
}

fn advance_state(app: &App) -> Option<ProcgenAdvance> {
    app.world().get_resource::<ProcgenAdvance>().copied()
}

fn request_next(app: &mut App) {
    let pending = app.world_mut().get_resource_mut::<PendingStepCommand>();
    assert!(
        pending.is_some(),
        "PendingStepCommand must exist while the stepper is engaged in Generation",
    );
    if let Some(mut pending) = pending {
        pending.request(StepCommand::Next);
    }
}

#[test]
fn a_held_generation_runs_no_stage() {
    let mut app = app_engaged_in_generation(FIXED_SEED);
    app.world_mut().insert_resource(ProcgenAdvance::Hold);

    app.update();

    assert_eq!(
        placements(&app),
        Some(0),
        "Hold must run no stage, so the driver is still present and has landed nothing",
    );
}

#[test]
fn one_stage_runs_exactly_one_stage_and_holds_again() {
    let mut app = app_engaged_in_generation(FIXED_SEED);
    app.world_mut().insert_resource(ProcgenAdvance::OneStage);

    app.update();

    assert_eq!(
        placements(&app),
        Some(1),
        "OneStage must run the first stage and stop, which lands the player placement",
    );
    assert_eq!(
        advance_state(&app),
        Some(ProcgenAdvance::Hold),
        "and it must write Hold back, so the next frame runs nothing without a new signal",
    );
}

#[test]
fn all_stages_runs_the_generation_out_in_one_update() {
    let mut app = app_engaged_in_generation(FIXED_SEED);
    app.world_mut().insert_resource(ProcgenAdvance::AllStages);

    app.update();

    assert_eq!(
        placements(&app),
        None,
        "AllStages must run every remaining stage, so the finish clears the driver away in the \
         same update",
    );
    assert_eq!(
        requests(&app),
        1,
        "and that finish must write the one battle-setup request",
    );
    assert!(
        !has_context(&app),
        "the same finish must clear BattleGenerationContext, or the next frame finds a context \
         with no driver and writes a second request",
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::Generation),
        "the machine is still in Generation after the finish, so the finish runs again next frame",
    );

    drain_requests(&mut app);
    for frame in 0..FRAMES_AFTER_FINISH {
        app.update();
        assert_eq!(
            requests(&app),
            0,
            "a finished generation must write no second battle-setup request; frame {frame} \
             wrote one, which sets a second battle up over the first",
        );
    }
}

#[test]
fn an_absent_advance_runs_the_generation_out_in_one_update() {
    let mut app = app_engaged_in_generation(FIXED_SEED);
    app.world_mut().remove_resource::<ProcgenAdvance>();

    app.update();

    assert_eq!(
        placements(&app),
        None,
        "absent means AllStages, which is what a battle with no stepper gets",
    );
    assert_eq!(
        requests(&app),
        1,
        "and that finish must write the one battle-setup request",
    );
}

#[test]
fn the_stepper_signals_nothing_while_no_command_is_pending() {
    let mut app = app_engaged_in_generation(FIXED_SEED);

    app.update();

    assert_eq!(
        placements(&app),
        Some(0),
        "an engaged stepper holds the generation until something asks it to move",
    );
}

#[test]
fn a_pending_next_signals_exactly_one_stage() {
    let mut app = app_engaged_in_generation(FIXED_SEED);
    request_next(&mut app);

    app.update();

    assert_eq!(
        placements(&app),
        Some(1),
        "StepCommand::Next must signal OneStage, which lands exactly one placement",
    );
}
