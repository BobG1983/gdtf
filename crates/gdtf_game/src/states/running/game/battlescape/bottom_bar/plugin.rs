use bevy::prelude::*;
use gdtf_battle_sim::prelude::BattleInProgress;
use gdtf_ui::themed::UiSystems;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::bottom_bar::{
        slots::BottomBarSlots,
        systems::{despawn_bottom_bar, opacify_bottom_bar, repad_bottom_bar, spawn_bottom_bar},
    },
};

pub(in crate::states::running::game::battlescape) struct GameBattleScapeBottomBarScenePlugin;

impl Plugin for GameBattleScapeBottomBarScenePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            OnEnter(BattleScapeState::BattleRunning),
            (
                BottomBarSlots::WeaponPanel,
                BottomBarSlots::ContextualPanel,
                BottomBarSlots::SelectCycle,
            )
                .chain()
                .after(spawn_bottom_bar),
        )
        .add_systems(OnEnter(BattleScapeState::BattleRunning), spawn_bottom_bar)
        .add_systems(OnExit(BattleScapeState::BattleRunning), despawn_bottom_bar)
        .add_systems(
            Update,
            (opacify_bottom_bar, repad_bottom_bar)
                .after(UiSystems::ApplyTheme)
                .run_if(resource_exists::<BattleInProgress>),
        );
    }
}
