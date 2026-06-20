use bevy::prelude::*;

use crate::states::{
    AfterMathState, BattleScapeState,
    running::game::battlescape::aftermath::{
        GameBattleScapeAfterMathAnimateInScenePlugin,
        GameBattleScapeAfterMathAnimateOutScenePlugin,
        GameBattleScapeAfterMathDisplayAftermathScenePlugin, systems::*,
    },
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
    app.add_systems(OnEnter(BattleScapeState::AfterMath), print_on_enter)
        .add_systems(OnExit(BattleScapeState::AfterMath), print_on_exit);
}

fn add_plugins(app: &mut App) {
    app.add_plugins(GameBattleScapeAfterMathAnimateInScenePlugin)
        .add_plugins(GameBattleScapeAfterMathDisplayAftermathScenePlugin)
        .add_plugins(GameBattleScapeAfterMathAnimateOutScenePlugin);
}

fn add_states(app: &mut App) {
    app.add_sub_state::<AfterMathState>();
}
