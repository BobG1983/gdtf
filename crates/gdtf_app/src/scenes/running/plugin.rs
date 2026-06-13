use bevy::prelude::*;

use crate::{
    scenes::running::{
        GameScenePlugin, MenuScenePlugin, OptionsScenePlugin, QuitScenePlugin, systems::*,
    },
    states::{AppState, RunningState},
};

pub(in crate::scenes) struct RunningScenePlugin;

impl Plugin for RunningScenePlugin {
    fn build(&self, app: &mut App) {
        add_states(app);
        add_plugins(app);
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(AppState::Running), print_on_enter)
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
