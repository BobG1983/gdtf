use bevy::prelude::*;

use crate::{
    scenes::init::{resources::InitComplete, systems::*},
    states::AppState,
};

// Initialization plugin for the GDTF app.
pub(in crate::scenes) struct InitScenePlugin;

impl Plugin for InitScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(AppState::Init), print_on_enter)
        .add_systems(
            FixedUpdate,
            init_complete
                .run_if(in_state(AppState::Init).and(not(resource_exists::<InitComplete>))),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(in_state(AppState::Init).and(resource_exists::<InitComplete>)),
        )
        .add_systems(OnExit(AppState::Init), (print_on_exit, cleanup));
}
