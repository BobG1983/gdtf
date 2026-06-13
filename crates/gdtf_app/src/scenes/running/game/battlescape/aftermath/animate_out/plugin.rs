use bevy::prelude::*;

use crate::{
    scenes::running::game::battlescape::aftermath::animate_out::{
        resources::AfterMathAnimateOutComplete, systems::*,
    },
    states::AfterMathState,
};

pub(in crate::scenes) struct GameBattleScapeAfterMathAnimateOutScenePlugin;

impl Plugin for GameBattleScapeAfterMathAnimateOutScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(AfterMathState::AnimateOut), print_on_enter)
        .add_systems(
            FixedUpdate,
            game_battlescape_aftermath_animate_out_complete.run_if(
                in_state(AfterMathState::AnimateOut)
                    .and(not(resource_exists::<AfterMathAnimateOutComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(
                in_state(AfterMathState::AnimateOut)
                    .and(resource_exists::<AfterMathAnimateOutComplete>),
            ),
        )
        .add_systems(OnExit(AfterMathState::AnimateOut), (print_on_exit, cleanup));
}
