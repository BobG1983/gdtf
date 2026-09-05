//! set through the authoritative live path.
/// file caps). `#[path]` because a test-crate ROOT resolves a bare `mod` beside itself in
#[path = "load_gangs_spawn_expected.rs"]
mod expected;

use std::collections::HashMap;

use bevy::{
    app::App,
    prelude::{Entity, NextState, World},
    state::state::State,
};
use cobalt_test_utils::{LoadTestAppBuilder, advance_until};
use gdtf_battle_sim::ganger::{GangRegistry, GangerName};
use gdtf_game::test_support::{AppState, BattleScapeState, RunningState};

fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

#[test]
fn real_skirmish_with_real_gangs_spawns_the_expected_set() {
    let mut app =
        LoadTestAppBuilder::new(gdtf_game::test_support::register_scenes_with_default_plugins)
            .starting_in(AppState::Load)
            .build();

    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Menu)
    });

    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });

    let gangs = app.world().get_resource::<GangRegistry>().cloned();
    assert!(
        gangs.is_some(),
        "the real Load flow must resolve a GangRegistry from the shipped gangs",
    );
    let Some(gangs) = gangs else {
        return;
    };

    let spawned = ganger_entities(app.world_mut());
    expected::assert_expected_set(app.world(), &spawned, &gangs);
}

fn ganger_entities(world: &mut World) -> HashMap<String, Entity> {
    let mut query = world.query::<(Entity, &GangerName)>();
    query
        .iter(world)
        .map(|(entity, name)| ((**name).clone(), entity))
        .collect()
}
