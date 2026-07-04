use bevy::prelude::*;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::generation::{
        battle_sim::BattleSimPlugin, loading_screen::LoadingScreenPlugin,
        resources::GenerationComplete,
    },
    scaffold::{
        SceneLabel, advance_state_to, log_scene_enter, log_scene_exit, remove_scoped_resource,
    },
};

pub(in crate::states) struct GameBattleScapeGenerationScenePlugin;

impl Plugin for GameBattleScapeGenerationScenePlugin {
    fn build(&self, app: &mut App) {
        // The render-free sim integration (E10.5 / GTW-207): seeds the battle RNG,
        // builds the battle from the authored situation on entry to Generation, and
        // gates `GenerationComplete` on REAL setup success (its presence-gated poll
        // replaces the previous unconditional no-op insert).
        app.add_plugins(BattleSimPlugin);
        // GTW-419: the LOADING SCREEN view — the themed full-viewport overlay shown while the
        // sim assembles the level + builds the battle, removed exactly on the transition to
        // `AnimateIn`. The `BattleReady`-gated `Generation → AnimateIn` transition below is
        // UNCHANGED; the screen only COVERS that phase so no partial-level frame is shown (AC2).
        app.add_plugins(LoadingScreenPlugin);
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    let label = SceneLabel::new("Game::BattleScape::Generation");
    app.add_systems(
        OnEnter(BattleScapeState::Generation),
        log_scene_enter(label),
    )
    .add_systems(
        FixedUpdate,
        // No scaffold marker-insert here: `GenerationComplete` is inserted by the
        // `BattleSimPlugin` poll on REAL setup success (E10.5 / GTW-207), so this
        // scene only registers the marker-gated advance + the exit remove.
        advance_state_to(BattleScapeState::AnimateIn).run_if(
            in_state(BattleScapeState::Generation).and_then(resource_exists::<GenerationComplete>),
        ),
    )
    .add_systems(
        OnExit(BattleScapeState::Generation),
        (
            log_scene_exit(label),
            remove_scoped_resource::<GenerationComplete>(),
        ),
    );
}
