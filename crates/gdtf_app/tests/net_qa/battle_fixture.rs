use std::sync::mpsc;

use bevy::{app::App, state::state::State};
use gdtf_app::test_support::{
    AppState, LoadedSituation, NetQaPlugin, RunningState, StartBattleRequested,
};
use gdtf_battle_sim::{
    effects::fields::FieldDefRegistry,
    test_support::{
        fixtures, test_armor_registry, test_gang_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::{CombatTuning, GangerStatTuning},
};
use gdtf_net_qa_transport::IncomingRequest;
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

pub(crate) const DRIVE_BUDGET: u32 = 128;

fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

pub(crate) fn menu_app_with_net_qa() -> (App, mpsc::Sender<IncomingRequest>) {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(GangerStatTuning::default());
    app.world_mut().insert_resource(test_weapon_registry());
    app.world_mut()
        .insert_resource(test_melee_weapon_registry());
    app.world_mut().insert_resource(test_armor_registry());
    app.world_mut().insert_resource(FieldDefRegistry::default());
    app.world_mut().insert_resource(test_gang_registry());
    app.world_mut()
        .insert_resource(LoadedSituation::new(fixtures::two_ganger()));

    let (tx, rx) = mpsc::channel();
    app.add_plugins(NetQaPlugin::with_channels(rx));

    let rested = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        DRIVE_BUDGET,
    );
    assert!(rested, "the harness must rest at RunningState::Menu");
    (app, tx)
}

pub(crate) fn request_battle(app: &mut App) {
    app.world_mut()
        .write_message(StartBattleRequested::new(None));
}
