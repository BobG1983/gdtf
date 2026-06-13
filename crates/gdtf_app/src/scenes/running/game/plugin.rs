use bevy::prelude::*;

use crate::{
    scenes::running::game::{
        GameBattleScapeScenePlugin, GameHiveScapeScenePlugin, GameSetupScenePlugin, systems::*,
    },
    states::{GameState, RunningState},
};

pub(in crate::scenes) struct GameScenePlugin;

impl Plugin for GameScenePlugin {
    fn build(&self, app: &mut App) {
        add_states(app);
        add_plugins(app);
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(RunningState::Game), print_on_enter)
        .add_systems(OnExit(RunningState::Game), print_on_exit);
}

fn add_plugins(app: &mut App) {
    app.add_plugins(GameSetupScenePlugin)
        .add_plugins(GameHiveScapeScenePlugin)
        .add_plugins(GameBattleScapeScenePlugin);
}

fn add_states(app: &mut App) {
    app.add_sub_state::<GameState>();
}
