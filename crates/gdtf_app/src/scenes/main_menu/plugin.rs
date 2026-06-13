use bevy::prelude::*;

use crate::{
    scenes::main_menu::systems::{print_on_enter, print_on_exit},
    states::AppState,
};

// Main menu scene plugin for the GDTF app.
pub(crate) struct MainMenuScenePlugin;

impl Plugin for MainMenuScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) -> &mut App {
    app.add_systems(OnEnter(AppState::MainMenu), print_on_enter)
        .add_systems(OnExit(AppState::MainMenu), print_on_exit)
}
