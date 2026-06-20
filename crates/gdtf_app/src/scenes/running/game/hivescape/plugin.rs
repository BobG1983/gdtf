use bevy::prelude::*;

use crate::{
    scenes::running::game::hivescape::{resources::HiveScapeComplete, systems::*},
    states::GameState,
};

pub(in crate::scenes) struct GameHiveScapeScenePlugin;

impl Plugin for GameHiveScapeScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(GameState::HiveScape), print_on_enter)
        .add_systems(
            FixedUpdate,
            game_hivescape_complete.run_if(
                in_state(GameState::HiveScape).and_then(not(resource_exists::<HiveScapeComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(
                in_state(GameState::HiveScape).and_then(resource_exists::<HiveScapeComplete>),
            ),
        )
        .add_systems(OnExit(GameState::HiveScape), (print_on_exit, cleanup));
}
