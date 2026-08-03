use bevy::prelude::*;

use crate::states::{
    AppState,
    init::resources::InitComplete,
    scaffold::{
        SceneLabel, advance_state_to, insert_completion_marker, log_scene_enter, log_scene_exit,
        remove_scoped_resource,
    },
};

pub(in crate::states) struct InitScenePlugin;

impl Plugin for InitScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Init");
    app.add_systems(OnEnter(AppState::Init), log_scene_enter(label))
        .add_systems(
            FixedUpdate,
            insert_completion_marker::<InitComplete>()
                .run_if(in_state(AppState::Init).and_then(not(resource_exists::<InitComplete>))),
        )
        .add_systems(
            FixedUpdate,
            advance_state_to(AppState::Load)
                .run_if(in_state(AppState::Init).and_then(resource_exists::<InitComplete>)),
        )
        .add_systems(
            OnExit(AppState::Init),
            (
                log_scene_exit(label),
                remove_scoped_resource::<InitComplete>(),
            ),
        );
}
