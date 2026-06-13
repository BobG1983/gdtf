use bevy::prelude::*;

use crate::{
    scenes::init::systems::{print_on_enter, print_on_exit},
    states::AppState,
};

// Initialization plugin for the GDTF app.
pub(crate) struct InitScenePlugin;

impl Plugin for InitScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) -> &mut App {
    app.add_systems(OnEnter(AppState::Init), print_on_enter)
        .add_systems(OnExit(AppState::Init), print_on_exit)
}
