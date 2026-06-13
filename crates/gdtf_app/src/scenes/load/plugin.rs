use bevy::prelude::*;

use crate::{
    scenes::load::{resources::LoadComplete, systems::*},
    states::AppState,
};

pub(in crate::scenes) struct LoadScenePlugin;

impl Plugin for LoadScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(AppState::Load), print_on_enter)
        .add_systems(
            FixedUpdate,
            load_complete
                .run_if(in_state(AppState::Load).and(not(resource_exists::<LoadComplete>))),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(in_state(AppState::Load).and(resource_exists::<LoadComplete>)),
        )
        .add_systems(OnExit(AppState::Load), (print_on_exit, cleanup));
}
