//! the authoritative render-free sim, consumed ONE-WAY by the app). The sim owns its
use bevy::prelude::*;
use gdtf_battle_sim::{battle::BattleSimPlugin as SimBattleSimPlugin, occupancy_sync::SimSystems};

use crate::states::{
    BattleScapeState, GameState,
    running::game::battlescape::generation::{
        battle_sim::systems::{
            gate_generation_complete, request_battle_setup, request_battle_teardown,
        },
        resources::GenerationComplete,
    },
};

pub(in crate::states::running::game::battlescape::generation) struct BattleSimPlugin;

impl Plugin for BattleSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SimBattleSimPlugin);
        #[cfg(feature = "dev_tools")]
        app.add_systems(
            OnEnter(BattleScapeState::Generation),
            request_battle_setup.run_if(crate::dev::procgen_stepper::battle_setup_runs_directly),
        );
        #[cfg(not(feature = "dev_tools"))]
        app.add_systems(OnEnter(BattleScapeState::Generation), request_battle_setup);
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
