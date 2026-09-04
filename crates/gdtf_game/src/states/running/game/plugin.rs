use bevy::prelude::*;

use crate::states::{
    GameState, RunningState,
    running::game::{GameBattleScapeScenePlugin, GameHiveScapeScenePlugin, GameSetupScenePlugin},
    scaffold::{SceneLabel, log_scene_enter, log_scene_exit},
};

pub(in crate::states) struct GameScenePlugin;

impl Plugin for GameScenePlugin {
    fn build(&self, app: &mut App) {
        add_states(app);
        add_plugins(app);
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Running::Game");
    app.add_systems(OnEnter(RunningState::Game), log_scene_enter(label))
        .add_systems(OnExit(RunningState::Game), log_scene_exit(label));
}

fn add_plugins(app: &mut App) {
    app.add_plugins(GameSetupScenePlugin)
        .add_plugins(GameHiveScapeScenePlugin)
        .add_plugins(GameBattleScapeScenePlugin);
}

fn add_states(app: &mut App) {
    app.add_sub_state::<GameState>();
}
