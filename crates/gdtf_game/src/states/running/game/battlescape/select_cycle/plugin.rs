use bevy::prelude::*;
use gdtf_battle_input::dispatch_act_intents;
use gdtf_battle_presenter::playback_caught_up;
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::{
        bottom_bar::{BottomBarSlots, spawn_bottom_bar},
        select_cycle::systems::{
            despawn_select_cycle, select_cycle_button_intents, spawn_select_cycle,
        },
    },
};

pub(in crate::states::running::game::battlescape) struct GameBattleScapeSelectCycleScenePlugin;

impl Plugin for GameBattleScapeSelectCycleScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(BattleScapeState::BattleRunning),
            spawn_select_cycle
                .in_set(BottomBarSlots::SelectCycle)
                .after(spawn_bottom_bar),
        )
        .add_systems(
            OnExit(BattleScapeState::BattleRunning),
            despawn_select_cycle,
        )
        .add_systems(
            Update,
            select_cycle_button_intents
                .before(dispatch_act_intents)
                .run_if(resource_exists::<BattleInProgress>.and_then(playback_caught_up)),
        );
    }
}
