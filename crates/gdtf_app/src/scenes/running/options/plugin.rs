use bevy::prelude::*;

use crate::{
    scenes::running::options::{resources::OptionsComplete, systems::*},
    states::RunningState,
};

pub(in crate::scenes) struct OptionsScenePlugin;

impl Plugin for OptionsScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(RunningState::Options), print_on_enter)
        .add_systems(
            FixedUpdate,
            options_complete.run_if(
                in_state(RunningState::Options).and_then(not(resource_exists::<OptionsComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(
                in_state(RunningState::Options).and_then(resource_exists::<OptionsComplete>),
            ),
        )
        .add_systems(OnExit(RunningState::Options), (print_on_exit, cleanup));
}
