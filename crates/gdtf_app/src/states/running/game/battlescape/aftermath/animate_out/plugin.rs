use bevy::prelude::*;

use crate::states::{
    AfterMathState, RunningState,
    running::game::battlescape::aftermath::animate_out::resources::AfterMathAnimateOutComplete,
    scaffold::{
        SceneLabel, advance_state_to, insert_completion_marker, log_scene_enter, log_scene_exit,
        remove_scoped_resource,
    },
};

pub(in crate::states) struct GameBattleScapeAfterMathAnimateOutScenePlugin;

impl Plugin for GameBattleScapeAfterMathAnimateOutScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Game::BattleScape::AfterMath::AnimateOut");
    app.add_systems(OnEnter(AfterMathState::AnimateOut), log_scene_enter(label))
        .add_systems(
            FixedUpdate,
            insert_completion_marker::<AfterMathAnimateOutComplete>().run_if(
                in_state(AfterMathState::AnimateOut)
                    .and_then(not(resource_exists::<AfterMathAnimateOutComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            advance_state_to(RunningState::Quit).run_if(
                in_state(AfterMathState::AnimateOut)
                    .and_then(resource_exists::<AfterMathAnimateOutComplete>),
            ),
        )
        .add_systems(
            OnExit(AfterMathState::AnimateOut),
            (
                log_scene_exit(label),
                remove_scoped_resource::<AfterMathAnimateOutComplete>(),
            ),
        );
}
