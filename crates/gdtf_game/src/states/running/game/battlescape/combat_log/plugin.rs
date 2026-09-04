use bevy::{
    ecs::{message::Messages, schedule::SystemCondition},
    prelude::*,
};
use gdtf_battle_presenter::{CombatLogEvent, CombatLogSystems};
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::combat_log::{
        systems::{
            animate_combat_log_height, append_combat_log, despawn_combat_log,
            fade_combat_log_lines, slide_combat_log_lines, spawn_combat_log,
        },
        tuning::register_combat_log_hot_ron,
    },
};

pub(in crate::states::running::game::battlescape) struct GameBattleScapeCombatLogScenePlugin;

impl Plugin for GameBattleScapeCombatLogScenePlugin {
    fn build(&self, app: &mut App) {
        register_combat_log_hot_ron(app);
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(
        Update,
        append_combat_log.after(CombatLogSystems::Forward).run_if(
            resource_exists::<BattleInProgress>
                .and_then(resource_exists::<Messages<CombatLogEvent>>),
        ),
    );

    app.add_systems(OnEnter(BattleScapeState::BattleRunning), spawn_combat_log)
        .add_systems(OnExit(BattleScapeState::BattleRunning), despawn_combat_log)
        .add_systems(
            Update,
            (
                fade_combat_log_lines,
                slide_combat_log_lines,
                animate_combat_log_height,
            ),
        );
}
