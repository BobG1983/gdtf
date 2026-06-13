use bevy::prelude::*;

use crate::{
    scenes::playing::systems::{print_on_enter, print_on_exit},
    states::AppState,
};

// Playing scene plugin for the GDTF app.
pub(crate) struct PlayingScenePlugin;

impl Plugin for PlayingScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) -> &mut App {
    app.add_systems(OnEnter(AppState::Playing), print_on_enter)
        .add_systems(OnExit(AppState::Playing), print_on_exit)
}
