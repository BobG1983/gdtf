use bevy::prelude::*;

use crate::{
    scenes::main_menu::{resources::MainMenuComplete, systems::*},
    states::AppState,
};

pub(in crate::scenes) struct MainMenuScenePlugin;

impl Plugin for MainMenuScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) -> &mut App {
    app.add_systems(OnEnter(AppState::MainMenu), print_on_enter)
        .add_systems(
            FixedUpdate,
            main_menu_complete
                .run_if(in_state(AppState::MainMenu).and(not(resource_exists::<MainMenuComplete>))),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(in_state(AppState::MainMenu).and(resource_exists::<MainMenuComplete>)),
        )
        .add_systems(OnExit(AppState::MainMenu), (print_on_exit, cleanup))
}
