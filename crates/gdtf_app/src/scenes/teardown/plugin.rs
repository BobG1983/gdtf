use bevy::prelude::*;

use crate::{
    scenes::teardown::systems::{print_on_enter, print_on_exit},
    states::AppState,
};

// Teardown scene plugin for the GDTF app.
pub(crate) struct TeardownScenePlugin;

impl Plugin for TeardownScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) -> &mut App {
    app.add_systems(OnEnter(AppState::Teardown), print_on_enter)
        .add_systems(OnExit(AppState::Teardown), print_on_exit)
}
