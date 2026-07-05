//! The plugin build ran inside the real scene stack (AC1).

use bevy::{app::App, prelude::*};
use gdtf_app::test_support::{AppState, GameState, RunningState};
use gdtf_battle_input::{GdtfBattleInputActive, InspectTarget};
use gdtf_battle_sim::tuning::CombatTuning;
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough to drive the deep walk into the battlescape, bounded so a
/// machine that never reaches the predicate fails instead of hanging.
const BUDGET: u32 = 96;

// ---------------------------------------------------------------------------------
// AC1 — the plugin build ran inside the real scene stack.
// ---------------------------------------------------------------------------------

/// Reads the current [`GameState`] if it is active.
fn game_state(app: &App) -> Option<GameState> {
    app.world()
        .get_resource::<State<GameState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Builds the headless app at [`AppState::Running`], seeding the persistent `Load`
/// resources the machine needs to traverse `Load` under `MinimalPlugins` (the
/// `presenter_foundation` / `battle_bootstrap` precedent): `default_theme()` +
/// `CombatTuning`. No `LoadedSituation` is seeded — Generation falls back to the
/// default (empty) situation, sufficient to reach `BattleScape`.
fn scene_stack_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app
}

/// Drives the app from [`AppState::Running`] down to the first update on which
/// [`GameState::BattleScape`] is active. Returns whether it was reached.
fn drive_to_battlescape(app: &mut App) -> bool {
    let reached_menu = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    if !reached_menu {
        return false;
    }
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Options);
    advance_until(
        app,
        |app| game_state(app) == Some(GameState::BattleScape),
        BUDGET,
    )
}

/// AC1 — `GameBattleScapeScenePlugin` adds `GdtfBattleInputPlugin`, and its `build`
/// actually runs inside the real scene stack: descending to `GameState::BattleScape`
/// leaves the `GdtfBattleInputActive` marker (and the `InspectTarget` resource the
/// plugin initialises) present in the world.
#[test]
fn battlescape_scene_runs_the_input_plugin_build() {
    let mut app = scene_stack_app();
    assert!(
        drive_to_battlescape(&mut app),
        "the walk should descend to GameState::BattleScape within {BUDGET} updates; last observed \
         GameState was {:?}",
        game_state(&app),
    );
    assert_eq!(
        game_state(&app),
        Some(GameState::BattleScape),
        "the walk must rest inside GameState::BattleScape",
    );
    assert!(
        app.world()
            .get_resource::<GdtfBattleInputActive>()
            .is_some(),
        "GdtfBattleInputPlugin::build must have run inside the real scene stack — the \
         GdtfBattleInputActive marker is present once BattleScape is active",
    );
    assert!(
        app.world().get_resource::<InspectTarget>().is_some(),
        "GdtfBattleInputPlugin must initialise the InspectTarget resource on build",
    );
}
