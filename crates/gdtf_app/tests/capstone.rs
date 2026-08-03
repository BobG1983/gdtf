//! `Menu → Game → … → BattleRunning`. `#![cfg(...)]` below keeps this file on the `net_qa`
#![cfg(all(debug_assertions, feature = "net_qa"))]

use std::sync::mpsc;

use bevy::{prelude::*, state::state::State};
use gdtf_app::test_support::{
    AimToggleButton, BattleScapeState, EndTurnButton, LevelDownButton, LevelUpButton,
    LoadedSituation, NetQaPlugin, RunningState, StanceKneelingButton, StanceProneButton,
    StanceStandingButton, StartBattleRequested,
};
use gdtf_battle_input::InspectTarget;
use gdtf_battle_presenter::WorldCamera;
use gdtf_battle_sim::{
    injuries::InjuryRegistry, situation::Situation, tuning::CombatTuning, weapon::WeaponRegistry,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

const BUDGET: u32 = 96;

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

fn seed_load(app: &mut App) {
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
}

#[test]
fn full_stack_composes_to_battle_running() {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .default_start()
        .build();
    seed_load(&mut app);
    let (_tx, rx) = mpsc::channel();
    app.add_plugins(NetQaPlugin::with_channels(rx));

    assert!(
        advance_until(
            &mut app,
            |app| running_state(app) == Some(RunningState::Menu),
            BUDGET,
        ),
        "the walk should reach RunningState::Menu within {BUDGET} updates; last observed \
         RunningState was {:?}",
        running_state(&app),
    );

    app.world_mut()
        .write_message(StartBattleRequested::new(None));
    app.update();

    assert!(
        advance_until(
            &mut app,
            |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
            BUDGET,
        ),
        "the full stack must compose and reach BattleRunning within {BUDGET} updates; \
         last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    let world_cameras = {
        let world = app.world_mut();
        let mut q = world.query::<&WorldCamera>();
        q.iter(world).count()
    };
    assert_eq!(
        world_cameras, 1,
        "exactly one WorldCamera must be present in BattleRunning (the S2 camera \
         lifecycle ran)",
    );

    assert!(
        app.world().get_resource::<InspectTarget>().is_some(),
        "the input crate's InspectTarget resource must be present (GdtfBattleInputPlugin \
         is wired into the real stack)",
    );

    assert_eq!(
        count_action_bar_buttons(&mut app),
        EXPECTED_ACTION_BAR_BUTTONS,
        "all {EXPECTED_ACTION_BAR_BUTTONS} action-bar buttons must be spawned in \
         BattleRunning",
    );

    app.update();
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::BattleRunning),
        "the stack must keep running in BattleRunning across an extra update without \
         panicking",
    );
}

const EXPECTED_ACTION_BAR_BUTTONS: usize = 7;

fn count_action_bar_buttons(app: &mut App) -> usize {
    count_marker::<StanceStandingButton>(app)
        + count_marker::<StanceKneelingButton>(app)
        + count_marker::<StanceProneButton>(app)
        + count_marker::<AimToggleButton>(app)
        + count_marker::<LevelUpButton>(app)
        + count_marker::<LevelDownButton>(app)
        + count_marker::<EndTurnButton>(app)
}

fn count_marker<M: Component>(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query_filtered::<Entity, With<M>>();
    q.iter(world).count()
}
