use bevy::prelude::*;

use crate::states::{
    AppState, RunningState,
    running::{GameScenePlugin, MenuScenePlugin, OptionsScenePlugin, QuitScenePlugin, systems::*},
    scaffold::{SceneLabel, log_scene_enter, log_scene_exit},
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
    let label = SceneLabel::new("Running");
    app.add_systems(
        OnEnter(AppState::Running),
        (log_scene_enter(label), spawn_ui_camera),
    )
    .add_systems(OnExit(AppState::Running), log_scene_exit(label));
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
