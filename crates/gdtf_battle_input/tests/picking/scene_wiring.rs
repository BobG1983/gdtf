use bevy::{app::App, prelude::*};
use gdtf_app::test_support::{AppState, GameState, RunningState};
use gdtf_battle_input::{GdtfBattleInputActive, InspectTarget};
use gdtf_battle_sim::tuning::CombatTuning;
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

fn game_state(app: &App) -> Option<GameState> {
    app.world()
        .get_resource::<State<GameState>>()
        .map(|state| *state.get())
}

fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

fn scene_stack_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(
        ONE_STEP_A_FRAME,
    ));
    app
}

fn drive_to_battlescape(app: &mut App) {
    advance_until(app, |app| running_state(app) == Some(RunningState::Menu));
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Options);
    advance_until(app, |app| running_state(app) == Some(RunningState::Options));
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(app, |app| game_state(app) == Some(GameState::BattleScape));
}

#[test]
fn battlescape_scene_runs_the_input_plugin_build() {
    let mut app = scene_stack_app();
    drive_to_battlescape(&mut app);
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
