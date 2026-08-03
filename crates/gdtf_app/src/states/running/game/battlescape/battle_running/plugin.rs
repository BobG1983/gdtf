use bevy::prelude::*;
use gdtf_battle_presenter::PresenterSystems;
use gdtf_battle_sim::occupancy_sync::SimSystems;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::battle_running::{resources::BattleRunningComplete, systems::*},
    scaffold::{SceneLabel, log_scene_enter, log_scene_exit, remove_scoped_resource},
};

pub(in crate::states) struct GameBattleScapeBattleRunningScenePlugin;

impl Plugin for GameBattleScapeBattleRunningScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Game::BattleScape::BattleRunning");
    app.add_systems(
        OnEnter(BattleScapeState::BattleRunning),
        log_scene_enter(label),
    )
    .add_systems(
        Update,
        end_battle_on_outcome.after(SimSystems::Simulate).run_if(
            in_state(BattleScapeState::BattleRunning)
                .and_then(not(resource_exists::<BattleRunningComplete>)),
        ),
    )
    // that frame (the correct "outcome decided" latch — unchanged). The BUG was that the
    .add_systems(
        Update,
        move_on.after(PresenterSystems::Draw).run_if(
            in_state(BattleScapeState::BattleRunning)
                .and_then(resource_exists::<BattleRunningComplete>),
        ),
    )
    .add_systems(
        OnExit(BattleScapeState::BattleRunning),
        (
            log_scene_exit(label),
            remove_scoped_resource::<BattleRunningComplete>(),
            remove_scoped_resource::<EndTransition>(),
        ),
    );
}
