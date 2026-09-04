use bevy::prelude::*;

use crate::states::{
    AppState,
    scaffold::{
        SceneLabel, insert_completion_marker, log_scene_enter, log_scene_exit,
        remove_scoped_resource,
    },
    teardown::{resources::TeardownComplete, systems::*},
};

pub(in crate::states) struct TeardownScenePlugin;

impl Plugin for TeardownScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Teardown");
    app.add_systems(OnEnter(AppState::Teardown), log_scene_enter(label))
        .add_systems(
            Update,
            (
                insert_completion_marker::<TeardownComplete>().run_if(
                    in_state(AppState::Teardown).and_then(not(resource_exists::<TeardownComplete>)),
                ),
                move_on.run_if(
                    in_state(AppState::Teardown).and_then(resource_exists::<TeardownComplete>),
                ),
            )
                .chain(),
        )
        .add_systems(
            OnExit(AppState::Teardown),
            (
                log_scene_exit(label),
                remove_scoped_resource::<TeardownComplete>(),
            ),
        );
}
