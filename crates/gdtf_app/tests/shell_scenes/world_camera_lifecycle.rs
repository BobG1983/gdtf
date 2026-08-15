//! World camera: one in `BattleScape`, despawns on exit, stays across sub-states.
use bevy::{
    camera::{Camera, visibility::RenderLayers},
    ecs::{entity::Entity, prelude::With},
    state::state::State,
};
use gdtf_app::test_support::{
    BattleRunningComplete, BattleScapeState, GameState, LoadedSituation, RunningState,
};
use gdtf_battle_presenter::WorldCamera;
use gdtf_battle_sim::{
    injuries::InjuryRegistry, situation::Situation, tuning::CombatTuning, weapon::WeaponRegistry,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

fn world_cameras(app: &mut bevy::app::App) -> Vec<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<WorldCamera>>()
        .iter(app.world())
        .collect()
}

fn game_state(app: &bevy::app::App) -> Option<GameState> {
    app.world()
        .get_resource::<State<GameState>>()
        .map(|state| *state.get())
}

fn battlescape_state(app: &bevy::app::App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

fn walk_app() -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .default_start()
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::equipment::attachments::AttachmentRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));
    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(
        ONE_STEP_A_FRAME,
    ));
    app
}

fn drive_into_battlescape(app: &mut bevy::app::App) {
    advance_until(app, |app| running_state(app) == Some(RunningState::Menu));
    app.world_mut()
        .resource_mut::<bevy::state::state::NextState<RunningState>>()
        .set(RunningState::Options);
    advance_until(app, |app| running_state(app) == Some(RunningState::Options));
    app.world_mut()
        .resource_mut::<bevy::state::state::NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(app, |app| game_state(app) == Some(GameState::BattleScape));
}

#[test]
fn world_camera_spawns_in_battlescape_and_despawns_on_exit() {
    let mut app = walk_app();
    drive_into_battlescape(&mut app);

    assert_eq!(
        world_cameras(&mut app).len(),
        1,
        "exactly one WorldCamera-marked Camera2d must exist while GameState::BattleScape is active",
    );

    app.world_mut().insert_resource(BattleRunningComplete);
    advance_until(&mut app, |app| {
        game_state(app).is_none_or(|state| state != GameState::BattleScape)
    });

    assert_eq!(
        world_cameras(&mut app).len(),
        0,
        "the WorldCamera must be despawned on OnExit(GameState::BattleScape)",
    );
}

#[test]
fn world_camera_survives_battlescape_substate_transitions() -> Result<(), &'static str> {
    let mut app = walk_app();
    drive_into_battlescape(&mut app);
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::Generation),
        "precondition: entering GameState::BattleScape rests in BattleScapeState::Generation",
    );

    let cameras_before = world_cameras(&mut app);
    assert_eq!(
        cameras_before.len(),
        1,
        "precondition: exactly one WorldCamera after entering GameState::BattleScape",
    );
    let camera = cameras_before
        .first()
        .copied()
        .ok_or("precondition: a WorldCamera entity must exist in GameState::BattleScape")?;

    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });

    let cameras_after = world_cameras(&mut app);
    assert_eq!(
        cameras_after.len(),
        1,
        "the WorldCamera count must be unchanged across BattleScapeState sub-state transitions; \
         it must not be despawned on a BattleScapeState OnExit",
    );
    assert!(
        app.world().get_entity(camera).is_ok(),
        "the exact WorldCamera entity spawned on entry to GameState::BattleScape must still exist \
         after the BattleScapeState walk toward BattleRunning — proving it spans GameState::\
         BattleScape, not a sub-state",
    );

    Ok(())
}

#[test]
fn world_camera_renders_below_ui_and_off_layer_zero() {
    let mut app = walk_app();
    drive_into_battlescape(&mut app);

    let world = app.world_mut();
    let mut query = world.query_filtered::<(&Camera, &RenderLayers), With<WorldCamera>>();
    let configs: Vec<(isize, bool)> = query
        .iter(world)
        .map(|(camera, layers)| (camera.order, layers.intersects(&RenderLayers::layer(0))))
        .collect();

    assert_eq!(
        configs.len(),
        1,
        "exactly one WorldCamera with a Camera + RenderLayers must exist in GameState::BattleScape",
    );
    let Some(&(order, intersects_layer_zero)) = configs.first() else {
        return;
    };
    assert_eq!(
        order, -1,
        "the WorldCamera must render at Camera.order == -1 (beneath the UI camera at the default 0)",
    );
    assert!(
        !intersects_layer_zero,
        "the WorldCamera's RenderLayers must NOT intersect layer 0 (the UI camera's default layer)",
    );
}
