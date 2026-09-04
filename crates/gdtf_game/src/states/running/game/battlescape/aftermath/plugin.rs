use bevy::prelude::*;

use crate::states::{
    AfterMathState, BattleScapeState,
    running::game::battlescape::aftermath::{
        GameBattleScapeAfterMathAnimateInScenePlugin,
        GameBattleScapeAfterMathAnimateOutScenePlugin,
        GameBattleScapeAfterMathDisplayAftermathScenePlugin,
    },
    scaffold::{SceneLabel, log_scene_enter, log_scene_exit},
};

pub(in crate::states) struct GameBattleScapeAfterMathScenePlugin;

impl Plugin for GameBattleScapeAfterMathScenePlugin {
    fn build(&self, app: &mut App) {
        add_states(app);
        add_plugins(app);
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Game::BattleScape::AfterMath");
    app.add_systems(OnEnter(BattleScapeState::AfterMath), log_scene_enter(label))
        .add_systems(OnExit(BattleScapeState::AfterMath), log_scene_exit(label));
}

fn add_plugins(app: &mut App) {
    app.add_plugins(GameBattleScapeAfterMathAnimateInScenePlugin)
        .add_plugins(GameBattleScapeAfterMathDisplayAftermathScenePlugin)
        .add_plugins(GameBattleScapeAfterMathAnimateOutScenePlugin);
}

fn add_states(app: &mut App) {
    app.add_sub_state::<AfterMathState>();
}
