use bevy::prelude::*;
use gdtf_battle_input::{
    dispatch_act_intents, reset_move_target_on_fire_mode_change, sync_fire_mode_on_select,
};
use gdtf_battle_presenter::{ActiveLevel, playback_caught_up};
use gdtf_battle_sim::prelude::BattleInProgress;
use gdtf_ui::{drive_switches, repaint_segments, select_segment_on_press, themed::UiSystems};

use crate::states::{
    BattleScapeState,
    running::game::battlescape::action_bar::systems::{
        action_bar_button_intents, aim_switch_flip_intent, despawn_action_bar, flee_button_pressed,
        mode_segment_write, nowrap_control_labels, rebuild_mode_segments, spawn_action_bar,
        stance_segment_intent, sync_aim_switch_state, sync_level_button_bounds,
        sync_mode_active_segment, sync_mode_tu_cost_lines, sync_stance_active_segment,
        tag_mode_segments, tag_stance_segments,
    },
};

pub(in crate::states::running::game::battlescape) struct GameBattleScapeActionBarScenePlugin;

impl Plugin for GameBattleScapeActionBarScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(BattleScapeState::BattleRunning), spawn_action_bar)
            .add_systems(OnExit(BattleScapeState::BattleRunning), despawn_action_bar)
            .add_systems(
                Update,
                action_bar_button_intents
                    .before(dispatch_act_intents)
                    .run_if(resource_exists::<BattleInProgress>.and_then(playback_caught_up)),
            )
            .add_systems(
                Update,
                sync_level_button_bounds
                    .before(action_bar_button_intents)
                    .run_if(resource_exists::<BattleInProgress>)
                    .run_if(resource_exists::<ActiveLevel>),
            )
            .add_systems(
                Update,
                flee_button_pressed.run_if(resource_exists::<BattleInProgress>),
            )
            .add_systems(
                Update,
                aim_switch_flip_intent
                    .after(drive_switches)
                    .before(dispatch_act_intents)
                    .run_if(resource_exists::<BattleInProgress>.and_then(playback_caught_up)),
            )
            .add_systems(
                Update,
                sync_aim_switch_state.run_if(resource_exists::<BattleInProgress>),
            )
            .add_systems(
                Update,
                stance_segment_intent
                    .after(select_segment_on_press)
                    .before(dispatch_act_intents)
                    .run_if(resource_exists::<BattleInProgress>.and_then(playback_caught_up)),
            )
            .add_systems(
                Update,
                (
                    tag_stance_segments,
                    sync_stance_active_segment.before(repaint_segments),
                )
                    .run_if(resource_exists::<BattleInProgress>),
            )
            .add_systems(
                Update,
                (
                    tag_mode_segments,
                    sync_mode_active_segment
                        .after(mode_segment_write)
                        .before(repaint_segments),
                )
                    .run_if(resource_exists::<BattleInProgress>),
            )
            .add_systems(
                Update,
                mode_segment_write
                    .after(select_segment_on_press)
                    .after(sync_fire_mode_on_select)
                    .before(reset_move_target_on_fire_mode_change)
                    .run_if(resource_exists::<BattleInProgress>.and_then(playback_caught_up)),
            )
            .add_systems(
                Update,
                rebuild_mode_segments
                    .after(UiSystems::ApplyTheme)
                    .run_if(resource_exists::<BattleInProgress>),
            )
            .add_systems(
                Update,
                sync_mode_tu_cost_lines
                    .after(rebuild_mode_segments)
                    .run_if(resource_exists::<BattleInProgress>),
            )
            .add_systems(
                Update,
                nowrap_control_labels
                    .after(UiSystems::ApplyTheme)
                    .run_if(resource_exists::<BattleInProgress>),
            );
    }
}
