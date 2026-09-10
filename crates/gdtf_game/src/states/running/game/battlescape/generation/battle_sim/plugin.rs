//! the authoritative render-free sim, consumed ONE-WAY by the app). The sim owns its
use bevy::prelude::*;
use gdtf_battle_sim::{
    battle::{BattleSimPlugin as SimBattleSimPlugin, setup_battle_on_request},
    occupancy_sync::SimSystems,
};

use crate::states::{
    BattleScapeState, GameState,
    running::game::battlescape::generation::{
        battle_sim::systems::{
            advance_battle_generation, begin_battle_generation, clear_battle_generation,
            finish_battle_generation, gate_generation_complete, request_battle_teardown,
        },
        resources::GenerationComplete,
    },
};

pub(in crate::states::running::game::battlescape::generation) struct BattleSimPlugin;

impl Plugin for BattleSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SimBattleSimPlugin);
        app.add_systems(
            OnEnter(BattleScapeState::Generation),
            begin_battle_generation,
        );
        app.add_systems(
            Update,
            (
                advance_battle_generation,
                finish_battle_generation
                    .after(advance_battle_generation)
                    .before(setup_battle_on_request),
            )
                .run_if(in_state(BattleScapeState::Generation)),
        );
        app.add_systems(
            OnExit(BattleScapeState::Generation),
            clear_battle_generation,
        );
        app.add_systems(
            Update,
            gate_generation_complete.after(SimSystems::Simulate).run_if(
                in_state(BattleScapeState::Generation)
                    .and_then(not(resource_exists::<GenerationComplete>)),
            ),
        )
        .add_systems(OnExit(GameState::BattleScape), request_battle_teardown);
    }
}
