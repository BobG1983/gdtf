use bevy::prelude::*;

use crate::{
    scenes::load::systems::{print_on_enter, print_on_exit},
    states::AppState,
};

// Load scene plugin for the GDTF app.
pub(crate) struct LoadScenePlugin;

impl Plugin for LoadScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) -> &mut App {
    app.add_systems(OnEnter(AppState::Load), print_on_enter)
        .add_systems(OnExit(AppState::Load), print_on_exit)
}
