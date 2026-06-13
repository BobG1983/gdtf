use bevy::prelude::*;

use crate::{
    scenes::running::game::battlescape::generation::{resources::GenerationComplete, systems::*},
    states::BattleScapeState,
};

pub(in crate::scenes) struct GameBattleScapeGenerationScenePlugin;

impl Plugin for GameBattleScapeGenerationScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(BattleScapeState::Generation), print_on_enter)
        .add_systems(
            FixedUpdate,
            game_battlescape_generation_complete.run_if(
                in_state(BattleScapeState::Generation)
                    .and(not(resource_exists::<GenerationComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(
                in_state(BattleScapeState::Generation).and(resource_exists::<GenerationComplete>),
            ),
        )
        .add_systems(
            OnExit(BattleScapeState::Generation),
            (print_on_exit, cleanup),
        );
}
