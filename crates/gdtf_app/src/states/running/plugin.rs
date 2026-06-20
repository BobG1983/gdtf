use bevy::prelude::*;

use crate::states::{
    AppState, RunningState,
    running::{GameScenePlugin, MenuScenePlugin, OptionsScenePlugin, QuitScenePlugin, systems::*},
};

pub(in crate::states) struct RunningScenePlugin;

impl Plugin for RunningScenePlugin {
    fn build(&self, app: &mut App) {
        add_states(app);
        add_plugins(app);
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    // The persistent UI camera is spawned on entry to `AppState::Running` and is
    // owned by the app shell (this scene plugin), not by `gdtf_ui`: its lifetime
    // is tied to the `Running` phase as a whole — every `RunningState` screen
    // renders against it — rather than to any one sub-state or widget. It carries
    // no scene-scoped despawn marker, so it outlives every `RunningState`
    // transition.
    app.add_systems(
        OnEnter(AppState::Running),
        (print_on_enter, spawn_ui_camera),
    )
    .add_systems(OnExit(AppState::Running), print_on_exit);
}

fn add_plugins(app: &mut App) {
    app.add_plugins(MenuScenePlugin)
        .add_plugins(GameScenePlugin)
        .add_plugins(OptionsScenePlugin)
        .add_plugins(QuitScenePlugin);
}

fn add_states(app: &mut App) {
    app.add_sub_state::<RunningState>();
}
