use bevy::prelude::*;

use crate::{
    scenes::running::menu::{resources::MenuComplete, systems::*},
    states::RunningState,
};

pub(in crate::scenes) struct MenuScenePlugin;

impl Plugin for MenuScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(RunningState::Menu), print_on_enter)
        .add_systems(
            FixedUpdate,
            menu_complete
                .run_if(in_state(RunningState::Menu).and(not(resource_exists::<MenuComplete>))),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(in_state(RunningState::Menu).and(resource_exists::<MenuComplete>)),
        )
        .add_systems(OnExit(RunningState::Menu), (print_on_exit, cleanup));
}
