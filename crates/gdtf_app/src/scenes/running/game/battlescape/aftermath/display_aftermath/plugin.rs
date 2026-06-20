use bevy::prelude::*;

use crate::{
    scenes::running::game::battlescape::aftermath::display_aftermath::{
        resources::DisplayAftermathComplete, systems::*,
    },
    states::AfterMathState,
};

pub(in crate::scenes) struct GameBattleScapeAfterMathDisplayAftermathScenePlugin;

impl Plugin for GameBattleScapeAfterMathDisplayAftermathScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(AfterMathState::DisplayAftermath), print_on_enter)
        .add_systems(
            FixedUpdate,
            game_battlescape_aftermath_display_aftermath_complete.run_if(
                in_state(AfterMathState::DisplayAftermath)
                    .and_then(not(resource_exists::<DisplayAftermathComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(
                in_state(AfterMathState::DisplayAftermath)
                    .and_then(resource_exists::<DisplayAftermathComplete>),
            ),
        )
        .add_systems(
            OnExit(AfterMathState::DisplayAftermath),
            (print_on_exit, cleanup),
        );
}
