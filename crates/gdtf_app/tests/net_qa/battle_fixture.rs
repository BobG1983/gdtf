use std::sync::mpsc;

use bevy::{app::App, state::state::State};
use gdtf_app::test_support::{
    AppState, BattleScapeState, LoadedSituation, NetQaPlugin, RunningState, StartBattleRequested,
};
use gdtf_battle_sim::{
    effects::fields::FieldDefRegistry,
    situation::Situation,
    test_support::{
        fixtures, test_armor_registry, test_gang_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::{CombatTuning, GangerStatTuning},
};
use gdtf_net_qa_transport::{IncomingRequest, Responder};
use gdtf_qa_protocol::{
    command::{CommandArgsRon, CommandName, RunOptions},
    message::{QaRequest, QaResponse, RunCommand},
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

pub(crate) const DRIVE_BUDGET: u32 = 128;

fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Hand one request to the router's inbox and keep the channel its reply will arrive on.
pub(crate) fn send(
    tx: &mpsc::Sender<IncomingRequest>,
    request: QaRequest,
) -> mpsc::Receiver<QaResponse> {
    let (responder, reply_rx) = Responder::channel();
    let sent = tx.send(IncomingRequest::new(request, responder));
    assert!(sent.is_ok(), "the router inbox must be open");
    reply_rx
}

/// A plain `Run` request for `name` with `arguments` and no riders.
pub(crate) fn run_request(name: &'static str, arguments: &str) -> QaRequest {
    QaRequest::Run(RunCommand::with_options(
        CommandName::from_static(name),
        CommandArgsRon::new(arguments.to_owned()),
        RunOptions::default(),
    ))
}

pub(crate) fn menu_app_with_net_qa() -> (App, mpsc::Sender<IncomingRequest>) {
    menu_app_with_situation(fixtures::two_ganger())
}

/// The same harness on a chosen battlefield, for a case the shipped two-ganger one cannot tell.
pub(crate) fn menu_app_with_situation(
    situation: Situation,
) -> (App, mpsc::Sender<IncomingRequest>) {
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
        .insert_resource(LoadedSituation::new(situation));

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

fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Drive the channel fixture down into a live, running battle.
pub(crate) fn drive_into_battle_running(app: &mut App) {
    request_battle(app);
    app.update();
    let reached = advance_until(
        app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        DRIVE_BUDGET,
    );
    assert!(
        reached,
        "the fixture must descend to BattleRunning before a battle command means anything; last \
         observed BattleScapeState was {:?}",
        battlescape_state(app),
    );
}
