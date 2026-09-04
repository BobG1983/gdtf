use bevy::prelude::*;

use crate::states::{
    AfterMathState,
    running::game::battlescape::aftermath::display_aftermath::resources::DisplayAftermathComplete,
    scaffold::{
        SceneLabel, advance_state_to, insert_completion_marker, log_scene_enter, log_scene_exit,
        remove_scoped_resource,
    },
};

pub(in crate::states) struct GameBattleScapeAfterMathDisplayAftermathScenePlugin;

impl Plugin for GameBattleScapeAfterMathDisplayAftermathScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Game::BattleScape::AfterMath::DisplayAftermath");
    app.add_systems(
        OnEnter(AfterMathState::DisplayAftermath),
        log_scene_enter(label),
    )
    .add_systems(
        FixedUpdate,
        insert_completion_marker::<DisplayAftermathComplete>().run_if(
            in_state(AfterMathState::DisplayAftermath)
                .and_then(not(resource_exists::<DisplayAftermathComplete>)),
        ),
    )
    .add_systems(
        FixedUpdate,
        advance_state_to(AfterMathState::AnimateOut).run_if(
            in_state(AfterMathState::DisplayAftermath)
                .and_then(resource_exists::<DisplayAftermathComplete>),
        ),
    )
    .add_systems(
        OnExit(AfterMathState::DisplayAftermath),
        (
            log_scene_exit(label),
            remove_scoped_resource::<DisplayAftermathComplete>(),
        ),
    );
}
