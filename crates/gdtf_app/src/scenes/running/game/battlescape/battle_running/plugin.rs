use bevy::prelude::*;

use crate::{
    scenes::running::game::battlescape::battle_running::{
        resources::BattleRunningComplete, systems::*,
    },
    states::BattleScapeState,
};

pub(in crate::scenes) struct GameBattleScapeBattleRunningScenePlugin;

impl Plugin for GameBattleScapeBattleRunningScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(BattleScapeState::BattleRunning), print_on_enter)
        .add_systems(
            FixedUpdate,
            game_battlescape_battle_running_complete.run_if(
                in_state(BattleScapeState::BattleRunning)
                    .and(not(resource_exists::<BattleRunningComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(
                in_state(BattleScapeState::BattleRunning)
                    .and(resource_exists::<BattleRunningComplete>),
            ),
        )
        .add_systems(
            OnExit(BattleScapeState::BattleRunning),
            (print_on_exit, cleanup),
        );
}
