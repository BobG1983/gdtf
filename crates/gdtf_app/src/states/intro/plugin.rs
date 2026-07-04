use bevy::prelude::*;

use crate::states::{
    AppState,
    intro::resources::IntroComplete,
    scaffold::{
        SceneLabel, advance_state_to, insert_completion_marker, log_scene_enter, log_scene_exit,
        remove_scoped_resource,
    },
};

pub(in crate::states) struct IntroScenePlugin;

impl Plugin for IntroScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Intro");
    app.add_systems(OnEnter(AppState::Intro), log_scene_enter(label))
        .add_systems(
            FixedUpdate,
            insert_completion_marker::<IntroComplete>()
                .run_if(in_state(AppState::Intro).and_then(not(resource_exists::<IntroComplete>))),
        )
        .add_systems(
            FixedUpdate,
            advance_state_to(AppState::Running)
                .run_if(in_state(AppState::Intro).and_then(resource_exists::<IntroComplete>)),
        )
        .add_systems(
            OnExit(AppState::Intro),
            (
                log_scene_exit(label),
                remove_scoped_resource::<IntroComplete>(),
            ),
        );
}
