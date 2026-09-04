use bevy::prelude::*;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::generation::{
        battle_sim::BattleSimPlugin, loading_screen::LoadingScreenPlugin,
        resources::GenerationComplete,
    },
    scaffold::{
        SceneLabel, advance_state_to, log_scene_enter, log_scene_exit, remove_scoped_resource,
    },
};

pub(in crate::states) struct GameBattleScapeGenerationScenePlugin;

impl Plugin for GameBattleScapeGenerationScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BattleSimPlugin);
        app.add_plugins(LoadingScreenPlugin);
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Game::BattleScape::Generation");
    app.add_systems(
        OnEnter(BattleScapeState::Generation),
        log_scene_enter(label),
    )
    .add_systems(
        FixedUpdate,
        advance_state_to(BattleScapeState::AnimateIn).run_if(
            in_state(BattleScapeState::Generation).and_then(resource_exists::<GenerationComplete>),
        ),
    )
    .add_systems(
        OnExit(BattleScapeState::Generation),
        (
            log_scene_exit(label),
            remove_scoped_resource::<GenerationComplete>(),
        ),
    );
}
