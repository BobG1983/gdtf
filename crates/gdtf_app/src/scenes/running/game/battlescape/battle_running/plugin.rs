use bevy::prelude::*;

use crate::{
    scenes::running::game::battlescape::battle_running::{
        resources::BattleRunningComplete, systems::*,
    },
    states::BattleScapeState,
};

pub(in crate::scenes) struct GameBattleScapeBattleRunningScenePlugin;

impl Plugin for GameBattleScapeBattleRunningScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(BattleScapeState::BattleRunning), print_on_enter)
        // The battlescape now PERSISTS in `BattleRunning` (GTW-236): the old 3-tick
        // turn-budget auto-exit is gone. `move_on` advances `BattleRunning → AnimateOut`
        // ONLY once the explicit end-signal marker `BattleRunningComplete` is present — the
        // victory census / flee button (sibling slices) are what insert it. Until one of
        // those lands the marker is never inserted, so the battle simply rests here.
        .add_systems(
            FixedUpdate,
            move_on.run_if(
                in_state(BattleScapeState::BattleRunning)
                    .and(resource_exists::<BattleRunningComplete>),
            ),
        )
        .add_systems(
            OnExit(BattleScapeState::BattleRunning),
            (print_on_exit, cleanup),
        );
}
