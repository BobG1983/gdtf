use bevy::prelude::*;

use crate::{
    scenes::running::game::battlescape::animate_out::{
        resources::BattleAnimateOutComplete, systems::*,
    },
    states::BattleScapeState,
};

pub(in crate::scenes) struct GameBattleScapeAnimateOutScenePlugin;

impl Plugin for GameBattleScapeAnimateOutScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(BattleScapeState::AnimateOut), print_on_enter)
        .add_systems(
            FixedUpdate,
            game_battlescape_animate_out_complete.run_if(
                in_state(BattleScapeState::AnimateOut)
                    .and(not(resource_exists::<BattleAnimateOutComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(
                in_state(BattleScapeState::AnimateOut)
                    .and(resource_exists::<BattleAnimateOutComplete>),
            ),
        )
        .add_systems(
            OnExit(BattleScapeState::AnimateOut),
            (print_on_exit, cleanup),
        );
}
