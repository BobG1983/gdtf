use bevy::prelude::*;
use gdtf_battle_input::{InputSystems, dispatch_act_intents};
use gdtf_battle_presenter::playback_caught_up;
use gdtf_battle_sim::prelude::BattleInProgress;
use gdtf_ui::themed::UiSystems;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::{
        bottom_bar::spawn_bottom_bar,
        weapon_panel::systems::{
            despawn_weapon_panel, fit_weapon_panel, reload_button_pressed, spawn_weapon_panel,
            update_weapon_panel,
        },
    },
};

pub(in crate::states::running::game::battlescape) struct GameBattleScapeWeaponPanelScenePlugin;

impl Plugin for GameBattleScapeWeaponPanelScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(BattleScapeState::BattleRunning),
            spawn_weapon_panel.after(spawn_bottom_bar),
        )
        .add_systems(
            OnExit(BattleScapeState::BattleRunning),
            despawn_weapon_panel,
        )
        .add_systems(
            Update,
            update_weapon_panel
                .after(InputSystems::Gather)
                .run_if(resource_exists::<BattleInProgress>),
        )
        .add_systems(
            Update,
            reload_button_pressed
                .before(dispatch_act_intents)
                .run_if(resource_exists::<BattleInProgress>.and_then(playback_caught_up)),
        )
        .add_systems(
            Update,
            fit_weapon_panel
                .after(UiSystems::ApplyTheme)
                .run_if(resource_exists::<BattleInProgress>),
        );
    }
}
