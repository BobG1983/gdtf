use bevy::state::state::State;
use gdtf_app::test_support::{AppState, GameState, RunningState, app_state};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};

use super::harness::*;

fn game_state(app: &bevy::app::App) -> Option<GameState> {
    app.world()
        .get_resource::<State<GameState>>()
        .map(|state| *state.get())
}

#[test]
fn running_hosts_menu() {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();

    app.update();

    assert_eq!(
        app_state(&app),
        AppState::Running,
        "starting_in(Running) should rest in AppState::Running after one update",
    );
    assert_eq!(
        running_state(&app),
        Some(RunningState::Menu),
        "AppState::Running must host its default child RunningState::Menu, not auto-advance past it",
    );
}

#[test]
fn game_hosts_setup() {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();

    assert!(
        drive_past_menu(&mut app),
        "the walk should reach RunningState::Menu within {WALK_BUDGET} updates; last observed \
         RunningState was {:?}",
        running_state(&app),
    );

    let reached = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Game),
        WALK_BUDGET,
    );
    assert!(
        reached,
        "the Menu →(player)→ Options → Game chain should reach RunningState::Game within \
         {WALK_BUDGET} updates; last observed RunningState was {:?}",
        running_state(&app),
    );

    assert_eq!(
        game_state(&app),
        Some(GameState::Setup),
        "entering RunningState::Game must activate its default child GameState::Setup",
    );
}
