use bevy::prelude::*;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::animate_in::resources::BattleAnimateInComplete,
    scaffold::{
        SceneLabel, advance_state_to, insert_completion_marker, log_scene_enter, log_scene_exit,
        remove_scoped_resource,
    },
};

pub(in crate::states) struct GameBattleScapeAnimateInScenePlugin;

impl Plugin for GameBattleScapeAnimateInScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Game::BattleScape::AnimateIn");
    app.add_systems(OnEnter(BattleScapeState::AnimateIn), log_scene_enter(label))
        .add_systems(
            FixedUpdate,
            insert_completion_marker::<BattleAnimateInComplete>().run_if(
                in_state(BattleScapeState::AnimateIn)
                    .and_then(not(resource_exists::<BattleAnimateInComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            advance_state_to(BattleScapeState::BattleRunning).run_if(
                in_state(BattleScapeState::AnimateIn)
                    .and_then(resource_exists::<BattleAnimateInComplete>),
            ),
        )
        .add_systems(
            OnExit(BattleScapeState::AnimateIn),
            (
                log_scene_exit(label),
                remove_scoped_resource::<BattleAnimateInComplete>(),
            ),
        );
}
