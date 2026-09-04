use bevy::{input_focus::directional_navigation::DirectionalNavigationMap, prelude::*};
use gdtf_battle_sim::prelude::BattleInProgress;
use gdtf_ui::focus_nav::FocusNavSystems;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::focus_nav::{
        bridge::{apply_focus_cancel, bridge_panel_focus_nav},
        outline::paint_focus_outline,
        topology::{clear_panel_nav_topology, rebuild_panel_nav_topology},
    },
};

pub(in crate::states::running::game::battlescape) struct GameBattleScapeFocusNavScenePlugin;

impl Plugin for GameBattleScapeFocusNavScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                rebuild_panel_nav_topology.before(FocusNavSystems::Apply),
                bridge_panel_focus_nav.before(FocusNavSystems::Apply),
                apply_focus_cancel.after(bridge_panel_focus_nav),
                paint_focus_outline,
            )
                .run_if(
                    resource_exists::<BattleInProgress>
                        .and_then(resource_exists::<DirectionalNavigationMap>),
                ),
        )
        .add_systems(
            OnExit(BattleScapeState::BattleRunning),
            clear_panel_nav_topology,
        );
    }
}
