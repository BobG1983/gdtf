use bevy::{app::App, prelude::NextState, state::state::State};
use gdtf_battle_sim::{
    ganger::GangerName, procgen::StagedProcgen, rng::BattleSeed, terrain::entity::TerrainIndex,
};
use gdtf_game::test_support::{
    AppState, BattleScapeState, PendingStepCommand, ProcgenStepperPlugin, RunningState, StepCommand,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

pub(crate) const FIXED_SEED: u64 = 0xC0FF_EE42;

pub(crate) fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

pub(crate) fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

pub(crate) fn terrain_fingerprint(app: &App) -> Option<usize> {
    app.world()
        .get_resource::<TerrainIndex>()
        .map(TerrainIndex::len)
}

pub(crate) fn deployed_ganger_count(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut query = world.query::<&GangerName>();
    query.iter(world).count()
}

pub(crate) fn app_ready_for_battle(seed: u64) -> App {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    app.world_mut().insert_resource(BattleSeed::new(seed));
    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(
        ONE_STEP_A_FRAME,
    ));

    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Menu)
    });
    app
}

pub(crate) fn drive_into_battle_running(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });
}

pub(crate) fn drive_stepper_to_done(app: &mut App) {
    while app.world().get_resource::<StagedProcgen>().is_some() {
        let pending = app.world_mut().get_resource_mut::<PendingStepCommand>();
        assert!(
            pending.is_some(),
            "PendingStepCommand must exist while the stepper is engaged in Generation",
        );
        if let Some(mut pending) = pending {
            pending.request(StepCommand::Next);
        }
        app.update();
    }
}

pub(crate) fn app_engaged_in_generation(seed: u64) -> App {
    let mut app = app_ready_for_battle(seed);
    app.add_plugins(ProcgenStepperPlugin::with_enabled(true));
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::Generation)
    });
    app
}
