use bevy::prelude::*;
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::states::{
    BattleScapeState, GameState,
    running::game::battlescape::{
        ContextualPanelPlugin, GameBattleScapeActionBarScenePlugin,
        GameBattleScapeAfterMathScenePlugin, GameBattleScapeAnimateInScenePlugin,
        GameBattleScapeAnimateOutScenePlugin, GameBattleScapeBattleRunningScenePlugin,
        GameBattleScapeBottomBarScenePlugin, GameBattleScapeCombatLogScenePlugin,
        GameBattleScapeFocusNavScenePlugin, GameBattleScapeGenerationScenePlugin,
        GameBattleScapeInspectPanelScenePlugin, GameBattleScapeSelectCycleScenePlugin,
        GameBattleScapeStatusPanelScenePlugin, GameBattleScapeWeaponPanelScenePlugin, systems::*,
    },
    scaffold::{SceneLabel, log_scene_enter, log_scene_exit},
};

pub(in crate::states) struct GameBattleScapeScenePlugin;

impl Plugin for GameBattleScapeScenePlugin {
    fn build(&self, app: &mut App) {
        add_states(app);
        add_plugins(app);
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Game::BattleScape");
    app.add_systems(OnEnter(GameState::BattleScape), log_scene_enter(label))
        .add_systems(OnExit(GameState::BattleScape), log_scene_exit(label))
        .add_systems(
            OnEnter(GameState::BattleScape),
            gdtf_battle_presenter::spawn_world_camera,
        )
        .add_systems(
            OnExit(GameState::BattleScape),
            gdtf_battle_presenter::despawn_world_camera,
        )
        .add_systems(
            Update,
            set_world_viewport.run_if(resource_exists::<BattleInProgress>),
        );
}

fn add_plugins(app: &mut App) {
    app.add_plugins(GameBattleScapeGenerationScenePlugin)
        .add_plugins(GameBattleScapeAnimateInScenePlugin)
        .add_plugins(GameBattleScapeBattleRunningScenePlugin)
        .add_plugins(GameBattleScapeAnimateOutScenePlugin)
        .add_plugins(GameBattleScapeAfterMathScenePlugin)
        .add_plugins(gdtf_battle_presenter::BattlePresenterPlugin::default())
        .add_plugins(gdtf_battle_input::GdtfBattleInputPlugin)
        .add_plugins(GameBattleScapeActionBarScenePlugin)
        .add_plugins(GameBattleScapeStatusPanelScenePlugin)
        .add_plugins(GameBattleScapeInspectPanelScenePlugin)
        .add_plugins(GameBattleScapeBottomBarScenePlugin)
        .add_plugins(GameBattleScapeWeaponPanelScenePlugin)
        .add_plugins(GameBattleScapeSelectCycleScenePlugin)
        .add_plugins(ContextualPanelPlugin)
        .add_plugins(GameBattleScapeFocusNavScenePlugin)
        .add_plugins(GameBattleScapeCombatLogScenePlugin);
}

fn add_states(app: &mut App) {
    app.add_sub_state::<BattleScapeState>();
}
