use bevy::prelude::*;
use gdtf_battle_input::InputSystems;
use gdtf_battle_presenter::PresenterSystems;
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::inspect_panel::{
        shadow::{
            ShownCoverLedger, ShownOccupancyGrid, promote_shown_cover, promote_shown_occupancy,
        },
        systems::{despawn_inspect_panel, spawn_inspect_panel, update_inspect_panel},
    },
};

pub(in crate::states::running::game::battlescape) struct GameBattleScapeInspectPanelScenePlugin;

impl Plugin for GameBattleScapeInspectPanelScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShownOccupancyGrid>()
            .init_resource::<ShownCoverLedger>()
            .add_systems(
                OnEnter(BattleScapeState::BattleRunning),
                spawn_inspect_panel,
            )
            .add_systems(
                OnExit(BattleScapeState::BattleRunning),
                despawn_inspect_panel,
            )
            .add_systems(
                Update,
                (promote_shown_occupancy, promote_shown_cover)
                    .after(PresenterSystems::Compose)
                    .run_if(resource_exists::<BattleInProgress>),
            )
            .add_systems(
                Update,
                update_inspect_panel
                    .after(InputSystems::Gather)
                    .after(PresenterSystems::Compose)
                    .after(promote_shown_occupancy)
                    .after(promote_shown_cover)
                    .run_if(resource_exists::<BattleInProgress>),
            );
    }
}
