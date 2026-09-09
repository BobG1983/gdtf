use bevy::state::state::State;
use cobalt_test_utils::{MinimalTestAppBuilder, advance_until};
use gdtf_game::test_support::{AppState, GameState, RunningState, app_state};

use super::harness::*;

fn game_state(app: &bevy::app::App) -> Option<GameState> {
    app.world()
        .get_resource::<State<GameState>>()
        .map(|state| *state.get())
}

#[test]
fn running_hosts_menu() {
    let mut app =
        MinimalTestAppBuilder::new_with_scene_support(gdtf_game::test_support::register_headless)
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
    let mut app =
        MinimalTestAppBuilder::new_with_scene_support(gdtf_game::test_support::register_headless)
            .starting_in(AppState::Running)
            .build();

    drive_past_menu(&mut app);

    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Game)
    });

    assert_eq!(
        game_state(&app),
        Some(GameState::Setup),
        "entering RunningState::Game must activate its default child GameState::Setup",
    );
}
