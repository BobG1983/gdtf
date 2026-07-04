use bevy::prelude::*;

use crate::states::{
    GameState,
    running::game::hivescape::resources::HiveScapeComplete,
    scaffold::{
        SceneLabel, advance_state_to, insert_completion_marker, log_scene_enter, log_scene_exit,
        remove_scoped_resource,
    },
};

pub(in crate::states) struct GameHiveScapeScenePlugin;

impl Plugin for GameHiveScapeScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Game::HiveScape");
    app.add_systems(OnEnter(GameState::HiveScape), log_scene_enter(label))
        .add_systems(
            FixedUpdate,
            insert_completion_marker::<HiveScapeComplete>().run_if(
                in_state(GameState::HiveScape).and_then(not(resource_exists::<HiveScapeComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            advance_state_to(GameState::BattleScape).run_if(
                in_state(GameState::HiveScape).and_then(resource_exists::<HiveScapeComplete>),
            ),
        )
        .add_systems(
            OnExit(GameState::HiveScape),
            (
                log_scene_exit(label),
                remove_scoped_resource::<HiveScapeComplete>(),
            ),
        );
}
