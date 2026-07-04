use bevy::prelude::*;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::animate_out::resources::BattleAnimateOutComplete,
    scaffold::{
        SceneLabel, advance_state_to, insert_completion_marker, log_scene_enter, log_scene_exit,
        remove_scoped_resource,
    },
};

pub(in crate::states) struct GameBattleScapeAnimateOutScenePlugin;

impl Plugin for GameBattleScapeAnimateOutScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Game::BattleScape::AnimateOut");
    app.add_systems(
        OnEnter(BattleScapeState::AnimateOut),
        log_scene_enter(label),
    )
    .add_systems(
        FixedUpdate,
        insert_completion_marker::<BattleAnimateOutComplete>().run_if(
            in_state(BattleScapeState::AnimateOut)
                .and_then(not(resource_exists::<BattleAnimateOutComplete>)),
        ),
    )
    .add_systems(
        FixedUpdate,
        advance_state_to(BattleScapeState::AfterMath).run_if(
            in_state(BattleScapeState::AnimateOut)
                .and_then(resource_exists::<BattleAnimateOutComplete>),
        ),
    )
    .add_systems(
        OnExit(BattleScapeState::AnimateOut),
        (
            log_scene_exit(label),
            remove_scoped_resource::<BattleAnimateOutComplete>(),
        ),
    );
}
