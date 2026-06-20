use bevy::prelude::*;

use crate::states::{
    RunningState,
    running::quit::{resources::QuitComplete, systems::*},
};

pub(in crate::states) struct QuitScenePlugin;

impl Plugin for QuitScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(RunningState::Quit), print_on_enter)
        .add_systems(
            FixedUpdate,
            quit_complete.run_if(
                in_state(RunningState::Quit).and_then(not(resource_exists::<QuitComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(in_state(RunningState::Quit).and_then(resource_exists::<QuitComplete>)),
        )
        .add_systems(OnExit(RunningState::Quit), (print_on_exit, cleanup));
}
