use bevy::prelude::*;

use crate::states::{
    RunningState,
    running::options::resources::OptionsComplete,
    scaffold::{
        SceneLabel, advance_state_to, insert_completion_marker, log_scene_enter, log_scene_exit,
        remove_scoped_resource,
    },
};

pub(in crate::states) struct OptionsScenePlugin;

impl Plugin for OptionsScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Running::Options");
    app.add_systems(OnEnter(RunningState::Options), log_scene_enter(label))
        .add_systems(
            FixedUpdate,
            insert_completion_marker::<OptionsComplete>().run_if(
                in_state(RunningState::Options).and_then(not(resource_exists::<OptionsComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            advance_state_to(RunningState::Game).run_if(
                in_state(RunningState::Options).and_then(resource_exists::<OptionsComplete>),
            ),
        )
        .add_systems(
            OnExit(RunningState::Options),
            (
                log_scene_exit(label),
                remove_scoped_resource::<OptionsComplete>(),
            ),
        );
}
