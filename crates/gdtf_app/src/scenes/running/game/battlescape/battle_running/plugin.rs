use bevy::prelude::*;
use gdtf_battle_sim::occupancy_sync::SimSystems;

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
        //
        // `end_battle_on_outcome` (GTW-239) is the SECOND inserter of that marker (the
        // victory census's outcome twin of `gate_generation_complete`): it reads the
        // sim-owned `BattleWon` / `BattleLost` outcome buffers and inserts
        // `BattleRunningComplete` on EITHER. It runs in `Update` (the outcome signals are
        // per-`Update` sim messages), ordered `.after(SimSystems::Simulate)` so an outcome
        // written by the sim's `check_outcome` THIS update is read THIS update (no
        // one-frame lag — `bevy-traps.md` #3), and is presence-gated
        // (`not(resource_exists::<BattleRunningComplete>)`) so a census that re-declares
        // the outcome each tick inserts the marker at most once.
        .add_systems(
            Update,
            end_battle_on_outcome.after(SimSystems::Simulate).run_if(
                in_state(BattleScapeState::BattleRunning)
                    .and_then(not(resource_exists::<BattleRunningComplete>)),
            ),
        )
        .add_systems(
            FixedUpdate,
            move_on.run_if(
                in_state(BattleScapeState::BattleRunning)
                    .and_then(resource_exists::<BattleRunningComplete>),
            ),
        )
        .add_systems(
            OnExit(BattleScapeState::BattleRunning),
            (print_on_exit, cleanup),
        );
}
