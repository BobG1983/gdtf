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
        .add_plugins(GameBattleScapeAfterMathScenePlugin)
        // The GTW-48 presenter seam (GTW-215): the VIEW that mirrors the sim. Its
        // `build` runs here when the scene plugins register; the default mode
        // builds the (empty this slice) CP437 renderer.
        .add_plugins(gdtf_battle_presenter::BattlePresenterPlugin::default());
}

fn add_states(app: &mut App) {
    app.add_sub_state::<BattleScapeState>();
}
