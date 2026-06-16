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
        .add_systems(OnExit(GameState::BattleScape), print_on_exit)
        // GTW-216: the SHARED world-camera lifecycle. The presenter exposes these as
        // `pub` param-only systems but cannot name `GameState` (it has no `gdtf_app`
        // dep), so the app registers them on the `GameState::BattleScape` boundary —
        // the same span as `request_battle_teardown` — so the camera survives the whole
        // `Generation → AnimateIn → BattleRunning → AnimateOut → AfterMath` walk and is
        // torn down only when the battle is left.
        .add_systems(
            OnEnter(GameState::BattleScape),
            gdtf_battle_presenter::spawn_world_camera,
        )
        .add_systems(
            OnExit(GameState::BattleScape),
            gdtf_battle_presenter::despawn_world_camera,
        );
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
