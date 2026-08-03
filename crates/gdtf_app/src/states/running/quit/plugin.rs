use bevy::prelude::*;

use crate::states::{
    AppState, RunningState,
    running::quit::resources::QuitComplete,
    scaffold::{
        SceneLabel, advance_state_to, insert_completion_marker, log_scene_enter, log_scene_exit,
        remove_scoped_resource,
    },
};

pub(in crate::states) struct QuitScenePlugin;

impl Plugin for QuitScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Running::Quit");
    app.add_systems(OnEnter(RunningState::Quit), log_scene_enter(label))
        .add_systems(
            FixedUpdate,
            insert_completion_marker::<QuitComplete>().run_if(
                in_state(RunningState::Quit).and_then(not(resource_exists::<QuitComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            advance_state_to(AppState::Teardown)
                .run_if(in_state(RunningState::Quit).and_then(resource_exists::<QuitComplete>)),
        )
        .add_systems(
            OnExit(RunningState::Quit),
            (
                log_scene_exit(label),
                remove_scoped_resource::<QuitComplete>(),
            ),
        );
}
