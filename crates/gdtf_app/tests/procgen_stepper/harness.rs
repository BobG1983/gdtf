//! Shared setup for the GTW-655 procgen-stepper suite: build the REAL Load flow, drive it to a
//! named checkpoint, and read the deterministic terrain fingerprint the suite compares across
//! the normal and stepper-engaged paths.

use bevy::{app::App, prelude::NextState, state::state::State};
use gdtf_app::test_support::{
    AppState, BattleScapeState, PendingStepCommand, ProcgenStepperPlugin, RunningState, StepCommand,
};
use gdtf_battle_sim::{
    ganger::GangerName, procgen::StagedProcgen, rng::BattleSeed, terrain::entity::TerrainIndex,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

/// A generous budget for the real `DefaultPlugins` async asset loads + the full state descent
/// under contention (the `procgen_battle.rs` precedent).
pub(crate) const BUDGET: u32 = 512;

/// The fixed seed every test in this suite injects, so the normal and stepper-engaged runs draw
/// identically and their terrain fingerprints are directly comparable.
pub(crate) const FIXED_SEED: u64 = 0xC0FF_EE42;

/// Reads the current [`RunningState`] if it is active.
pub(crate) fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Reads the current [`BattleScapeState`] if it is active.
pub(crate) fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// The battle's terrain-entity count — the deterministic fingerprint this suite compares across
/// the normal and stepper-engaged runs. `None` if no battle has set up (no `TerrainIndex`).
pub(crate) fn terrain_fingerprint(app: &App) -> Option<usize> {
    app.world()
        .get_resource::<TerrainIndex>()
        .map(TerrainIndex::len)
}

/// The count of deployed ganger entities in the battle — the DEPLOYMENT fingerprint the GTW-765
/// test compares across the normal and stepper-engaged paths. `terrain_fingerprint` counts only
/// terrain, so it could not catch a battle whose terrain generated but whose roster never
/// deployed; this counts the [`GangerName`] every spawned ganger carries. Takes `&mut App`
/// because building a `QueryState` needs `&mut World`.
pub(crate) fn deployed_ganger_count(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut query = world.query::<&GangerName>();
    query.iter(world).count()
}

/// Build the REAL Load flow (populating the prefab / theme / terrain registries from shipped
/// content) with `seed` pre-injected as the [`BattleSeed`] override, and drive it to
/// `RunningState::Menu`.
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

/// Drive `app` from the menu into the battle and assert it reaches `BattleRunning`.
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

/// Drive the ENGAGED stepper one `Next` press at a time — through the SAME
/// [`PendingStepCommand`] latch the egui panel's Next button writes to — until its
/// [`StagedProcgen`] resource is gone (the drive finished and `finish_stepper_drive` removed
/// it), bounded by [`BUDGET`]. The GTW-732 per-PREFAB granularity makes the step count vary by
/// seed, so the caller drives to DONE rather than a fixed count (the old `for _ in 0..3`
/// per-STAGE loop no longer reaches completion once Fill is stepped per prefab).
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

/// Build the REAL Load flow with the stepper plugin FORCED enabled (bypassing the process-global
/// `GDTF_PROCGEN_STEPPER` env var, which would race any other test thread reading it), start the
/// battle, and advance until the drive has engaged (`BattleScapeState::Generation` reached).
/// Shared setup for the Skip / Auto app-wiring tests and the step-equivalence test.
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
