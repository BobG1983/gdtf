//! Shared setup for the GTW-655 procgen-stepper suite: build the REAL Load flow, drive it to a
//! named checkpoint, and read the deterministic terrain fingerprint the suite compares across
//! the normal and stepper-engaged paths.

use bevy::{app::App, prelude::NextState, state::state::State};
use gdtf_app::test_support::{AppState, BattleScapeState, ProcgenStepperPlugin, RunningState};
use gdtf_battle_sim::{rng::BattleSeed, terrain::entity::TerrainIndex};
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
