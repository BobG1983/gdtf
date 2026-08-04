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
use gdtf_app::test_support::{AppState, BattleScapeState, RunningState};
use gdtf_battle_sim::ganger::{GangRegistry, GangerName};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

const BUDGET: u32 = 512;

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
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    let reached_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    assert!(
        reached_menu,
        "the real Load flow must reach RunningState::Menu within {BUDGET} updates; last observed \
         RunningState was {:?}",
        running_state(&app),
    );

    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    let reached_battle = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        reached_battle,
        "the real skirmish must generate + DEPLOY its roster and reach BattleRunning within \
         {BUDGET} updates (a missing gang ref would leave zero gangers and never reach it); last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

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
