use bevy::prelude::*;
use gdtf_battle_input::InputSystems;
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::status_panel::{
        stability_readout::update_stability_readout,
        systems::{despawn_status_panel, spawn_status_panel, update_status_panel},
    },
};

pub(in crate::states::running::game::battlescape) struct GameBattleScapeStatusPanelScenePlugin;

impl Plugin for GameBattleScapeStatusPanelScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(BattleScapeState::BattleRunning), spawn_status_panel)
            .add_systems(
                OnExit(BattleScapeState::BattleRunning),
                despawn_status_panel,
            )
            .add_systems(
                Update,
                (update_status_panel, update_stability_readout)
                    .after(InputSystems::Gather)
                    .run_if(resource_exists::<BattleInProgress>),
            );
    }
}
