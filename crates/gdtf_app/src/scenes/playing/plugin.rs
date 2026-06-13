use bevy::prelude::*;

use crate::{
    scenes::playing::{resources::PlayingComplete, systems::*},
    states::AppState,
};

pub(in crate::scenes) struct PlayingScenePlugin;

impl Plugin for PlayingScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) -> &mut App {
    app.add_systems(OnEnter(AppState::Playing), print_on_enter)
        .add_systems(
            FixedUpdate,
            playing_complete
                .run_if(in_state(AppState::Playing).and(not(resource_exists::<PlayingComplete>))),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(in_state(AppState::Playing).and(resource_exists::<PlayingComplete>)),
        )
        .add_systems(OnExit(AppState::Playing), (print_on_exit, cleanup))
}
