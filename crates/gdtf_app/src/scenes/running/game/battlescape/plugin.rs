use bevy::prelude::*;

use crate::{
    scenes::running::game::battlescape::{
        GameBattleScapeAfterMathScenePlugin, GameBattleScapeAnimateInScenePlugin,
        GameBattleScapeAnimateOutScenePlugin, GameBattleScapeBattleRunningScenePlugin,
        GameBattleScapeGenerationScenePlugin, systems::*,
    },
    states::{BattleScapeState, GameState},
};

pub(in crate::scenes) struct GameBattleScapeScenePlugin;

impl Plugin for GameBattleScapeScenePlugin {
    fn build(&self, app: &mut App) {
        add_states(app);
        add_plugins(app);
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(GameState::BattleScape), print_on_enter)
        .add_systems(OnExit(GameState::BattleScape), print_on_exit);
}

fn add_plugins(app: &mut App) {
    app.add_plugins(GameBattleScapeGenerationScenePlugin)
        .add_plugins(GameBattleScapeAnimateInScenePlugin)
        .add_plugins(GameBattleScapeBattleRunningScenePlugin)
        .add_plugins(GameBattleScapeAnimateOutScenePlugin)
        .add_plugins(GameBattleScapeAfterMathScenePlugin);
}

fn add_states(app: &mut App) {
    app.add_sub_state::<BattleScapeState>();
}
