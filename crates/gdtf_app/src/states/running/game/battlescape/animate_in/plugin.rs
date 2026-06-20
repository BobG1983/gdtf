use bevy::prelude::*;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::animate_in::{resources::BattleAnimateInComplete, systems::*},
};

pub(in crate::states) struct GameBattleScapeAnimateInScenePlugin;

impl Plugin for GameBattleScapeAnimateInScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(BattleScapeState::AnimateIn), print_on_enter)
        .add_systems(
            FixedUpdate,
            game_battlescape_animate_in_complete.run_if(
                in_state(BattleScapeState::AnimateIn)
                    .and_then(not(resource_exists::<BattleAnimateInComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(
                in_state(BattleScapeState::AnimateIn)
                    .and_then(resource_exists::<BattleAnimateInComplete>),
            ),
        )
        .add_systems(
            OnExit(BattleScapeState::AnimateIn),
            (print_on_exit, cleanup),
        );
}
