use bevy::prelude::*;

use crate::{
    scenes::running::game::battlescape::aftermath::display_after::{
        resources::DisplayAfterComplete, systems::*,
    },
    states::AfterMathState,
};

pub(in crate::scenes) struct GameBattleScapeAfterMathDisplayAfterScenePlugin;

impl Plugin for GameBattleScapeAfterMathDisplayAfterScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(AfterMathState::DisplayAftermath), print_on_enter)
        .add_systems(
            FixedUpdate,
            game_battlescape_aftermath_display_after_complete.run_if(
                in_state(AfterMathState::DisplayAftermath)
                    .and(not(resource_exists::<DisplayAfterComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(
                in_state(AfterMathState::DisplayAftermath)
                    .and(resource_exists::<DisplayAfterComplete>),
            ),
        )
        .add_systems(
            OnExit(AfterMathState::DisplayAftermath),
            (print_on_exit, cleanup),
        );
}
