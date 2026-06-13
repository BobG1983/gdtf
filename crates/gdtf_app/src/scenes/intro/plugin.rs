use bevy::prelude::*;

use crate::{
    scenes::intro::systems::{print_on_enter, print_on_exit},
    states::AppState,
};

// Intro scene plugin for the GDTF app.
pub(crate) struct IntroScenePlugin;

impl Plugin for IntroScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) -> &mut App {
    app.add_systems(OnEnter(AppState::Intro), print_on_enter)
        .add_systems(OnExit(AppState::Intro), print_on_exit)
}
