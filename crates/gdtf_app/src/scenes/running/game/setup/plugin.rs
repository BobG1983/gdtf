use bevy::prelude::*;

use crate::{
    scenes::running::game::setup::{resources::SetupComplete, systems::*},
    states::GameState,
};

pub(in crate::scenes) struct GameSetupScenePlugin;

impl Plugin for GameSetupScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(GameState::Setup), print_on_enter)
        .add_systems(
            FixedUpdate,
            game_setup_complete
                .run_if(in_state(GameState::Setup).and_then(not(resource_exists::<SetupComplete>))),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(in_state(GameState::Setup).and_then(resource_exists::<SetupComplete>)),
        )
        .add_systems(OnExit(GameState::Setup), (print_on_exit, cleanup));
}
