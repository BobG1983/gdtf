use bevy::prelude::*;

use crate::{
    scenes::intro::{resources::IntroComplete, systems::*},
    states::AppState,
};

pub(in crate::scenes) struct IntroScenePlugin;

impl Plugin for IntroScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(AppState::Intro), print_on_enter)
        .add_systems(
            FixedUpdate,
            intro_complete
                .run_if(in_state(AppState::Intro).and_then(not(resource_exists::<IntroComplete>))),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(in_state(AppState::Intro).and_then(resource_exists::<IntroComplete>)),
        )
        .add_systems(OnExit(AppState::Intro), (print_on_exit, cleanup));
}
