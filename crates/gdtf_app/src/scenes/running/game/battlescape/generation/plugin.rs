use bevy::prelude::*;

use crate::{
    scenes::running::game::battlescape::generation::{
        battle_sim::BattleSimPlugin, resources::GenerationComplete, systems::*,
    },
    states::BattleScapeState,
};

pub(in crate::scenes) struct GameBattleScapeGenerationScenePlugin;

impl Plugin for GameBattleScapeGenerationScenePlugin {
    fn build(&self, app: &mut App) {
        // The render-free sim integration (E10.5 / GTW-207): seeds the battle RNG,
        // builds the battle from the authored situation on entry to Generation, and
        // gates `GenerationComplete` on REAL setup success (its presence-gated poll
        // replaces the previous unconditional no-op insert).
        app.add_plugins(BattleSimPlugin);
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(BattleScapeState::Generation), print_on_enter)
        .add_systems(
            FixedUpdate,
            move_on.run_if(
                in_state(BattleScapeState::Generation)
                    .and_then(resource_exists::<GenerationComplete>),
            ),
        )
        .add_systems(
            OnExit(BattleScapeState::Generation),
            (print_on_exit, cleanup),
        );
}
