use bevy::{app::App, prelude::NextState, state::state::State};
use gdtf_app::test_support::{
    AppState, BattleScapeState, PendingStepCommand, ProcgenStepperPlugin, RunningState, StepCommand,
};
use gdtf_battle_sim::{
    ganger::GangerName, procgen::StagedProcgen, rng::BattleSeed, terrain::entity::TerrainIndex,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

pub(crate) const BUDGET: u32 = 512;

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

    let reached_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    assert!(
        reached_menu,
        "the REAL Load flow must reach RunningState::Menu within {BUDGET} updates; last \
         observed RunningState was {:?}",
        running_state(&app),
    );
    app
}

pub(crate) fn drive_into_battle_running(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    let reached_running = advance_until(
        app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        reached_running,
        "the battle must reach BattleRunning within {BUDGET} updates; last observed \
         BattleScapeState was {:?}",
        battlescape_state(app),
    );
}

pub(crate) fn drive_stepper_to_done(app: &mut App) {
    for _ in 0..BUDGET {
        if app.world().get_resource::<StagedProcgen>().is_none() {
            return;
        }
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
    let reached_generation = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::Generation),
        BUDGET,
    );
    assert!(
        reached_generation,
        "the stepper-engaged path must reach BattleScapeState::Generation (request_battle_setup \
         gated off, engage_stepper driving instead); last observed state was {:?}",
        battlescape_state(&app),
    );
    app
}
