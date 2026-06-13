use bevy::prelude::*;

use crate::{
    scenes::teardown::{resources::TeardownComplete, systems::*},
    states::AppState,
};

// Teardown scene plugin for the GDTF app.
pub(in crate::scenes) struct TeardownScenePlugin;

impl Plugin for TeardownScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) -> &mut App {
    app.add_systems(OnEnter(AppState::Teardown), print_on_enter)
        .add_systems(
            FixedUpdate,
            teardown_complete
                .run_if(in_state(AppState::Teardown).and(not(resource_exists::<TeardownComplete>))),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(in_state(AppState::Teardown).and(resource_exists::<TeardownComplete>)),
        )
        .add_systems(OnExit(AppState::Teardown), (print_on_exit, cleanup))
}
