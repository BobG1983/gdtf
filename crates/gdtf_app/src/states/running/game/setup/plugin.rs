use bevy::prelude::*;

use crate::states::{
    GameState,
    running::game::setup::resources::SetupComplete,
    scaffold::{
        SceneLabel, advance_state_to, insert_completion_marker, log_scene_enter, log_scene_exit,
        remove_scoped_resource,
    },
};

pub(in crate::states) struct GameSetupScenePlugin;

impl Plugin for GameSetupScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Game::Setup");
    app.add_systems(OnEnter(GameState::Setup), log_scene_enter(label))
        .add_systems(
            FixedUpdate,
            insert_completion_marker::<SetupComplete>()
                .run_if(in_state(GameState::Setup).and_then(not(resource_exists::<SetupComplete>))),
        )
        .add_systems(
            FixedUpdate,
            advance_state_to(GameState::HiveScape)
                .run_if(in_state(GameState::Setup).and_then(resource_exists::<SetupComplete>)),
        )
        .add_systems(
            OnExit(GameState::Setup),
            (
                log_scene_exit(label),
                remove_scoped_resource::<SetupComplete>(),
            ),
        );
}
