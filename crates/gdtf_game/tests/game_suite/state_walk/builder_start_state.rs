//! The shared minimal builder's `starting_in` drives this app's own state machine.
use bevy::state::state::State;
use cobalt_test_utils::MinimalTestAppBuilder;
use gdtf_game::test_support::{AppState, RunningState};

#[test]
fn starting_in_running_enters_running_menu_after_one_update() {
    // — entering `Running` enters `Menu`, whose `spawn_menu` now authors its tree
    let mut app =
        MinimalTestAppBuilder::new_with_scene_support(gdtf_game::test_support::register_headless)
            .starting_in(AppState::Running)
            .build();

    app.update();

    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::Running,
        "AppState should be Running after one update",
    );
    assert_eq!(
        app.world().resource::<State<RunningState>>().get(),
        &RunningState::Menu,
        "RunningState sub-state should default to Menu under AppState::Running",
    );
}
