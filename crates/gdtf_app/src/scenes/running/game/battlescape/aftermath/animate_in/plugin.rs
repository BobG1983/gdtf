use bevy::prelude::*;

use crate::{
    scenes::running::game::battlescape::aftermath::animate_in::{
        resources::AfterMathAnimateInComplete, systems::*,
    },
    states::AfterMathState,
};

pub(in crate::scenes) struct GameBattleScapeAfterMathAnimateInScenePlugin;

impl Plugin for GameBattleScapeAfterMathAnimateInScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(AfterMathState::AnimateIn), print_on_enter)
        .add_systems(
            FixedUpdate,
            game_battlescape_aftermath_animate_in_complete.run_if(
                in_state(AfterMathState::AnimateIn)
                    .and(not(resource_exists::<AfterMathAnimateInComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(
                in_state(AfterMathState::AnimateIn)
                    .and(resource_exists::<AfterMathAnimateInComplete>),
            ),
        )
        .add_systems(OnExit(AfterMathState::AnimateIn), (print_on_exit, cleanup));
}
