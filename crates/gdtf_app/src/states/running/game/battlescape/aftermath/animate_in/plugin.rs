use bevy::prelude::*;

use crate::states::{
    AfterMathState,
    running::game::battlescape::aftermath::animate_in::resources::AfterMathAnimateInComplete,
    scaffold::{
        SceneLabel, advance_state_to, insert_completion_marker, log_scene_enter, log_scene_exit,
        remove_scoped_resource,
    },
};

pub(in crate::states) struct GameBattleScapeAfterMathAnimateInScenePlugin;

impl Plugin for GameBattleScapeAfterMathAnimateInScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Game::BattleScape::AfterMath::AnimateIn");
    app.add_systems(OnEnter(AfterMathState::AnimateIn), log_scene_enter(label))
        .add_systems(
            FixedUpdate,
            insert_completion_marker::<AfterMathAnimateInComplete>().run_if(
                in_state(AfterMathState::AnimateIn)
                    .and_then(not(resource_exists::<AfterMathAnimateInComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            advance_state_to(AfterMathState::DisplayAftermath).run_if(
                in_state(AfterMathState::AnimateIn)
                    .and_then(resource_exists::<AfterMathAnimateInComplete>),
            ),
        )
        .add_systems(
            OnExit(AfterMathState::AnimateIn),
            (
                log_scene_exit(label),
                remove_scoped_resource::<AfterMathAnimateInComplete>(),
            ),
        );
}
