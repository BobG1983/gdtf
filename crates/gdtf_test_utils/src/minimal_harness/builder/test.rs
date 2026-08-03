use bevy::state::state::State;
use gdtf_app::test_support::{AppState, RunningState};

use super::*;

#[test]
fn starting_in_running_enters_running_menu_after_one_update() {
    // GTW-322 — entering `Running` enters `Menu`, whose `spawn_menu` now authors its tree
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
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
