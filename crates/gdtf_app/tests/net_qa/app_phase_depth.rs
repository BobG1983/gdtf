use std::sync::mpsc;

use bevy::{app::App, state::state::State};
use gdtf_app::test_support::{AppState, BattleScapeState, GameState, RunningState};
use gdtf_net_qa_transport::{IncomingRequest, Responder};
use gdtf_qa_protocol::{
    command::{CommandArgsRon, CommandName, CommandOutcome, RunOptions},
    message::{QaRequest, QaResponse, RunCommand},
};
use gdtf_test_utils::advance_until;

use super::{
    battle_fixture::{DRIVE_BUDGET, menu_app_with_net_qa, request_battle},
    command_exchange::APP_PHASE,
};

fn send(tx: &mpsc::Sender<IncomingRequest>, request: QaRequest) -> mpsc::Receiver<QaResponse> {
    let (responder, reply_rx) = Responder::channel();
    let sent = tx.send(IncomingRequest::new(request, responder));
    assert!(sent.is_ok(), "the router inbox must be open");
    reply_rx
}

fn read_phase(app: &mut App, tx: &mpsc::Sender<IncomingRequest>) -> String {
    let reply = send(
        tx,
        QaRequest::Run(RunCommand::with_options(
            CommandName::from_static(APP_PHASE),
            CommandArgsRon::new("()".to_owned()),
            RunOptions::default(),
        )),
    );
    app.update();
    app.update();
    let Ok(QaResponse::Outcome(CommandOutcome::Ran { reply, .. })) = reply.try_recv() else {
        unreachable!("a plain app.phase call must RUN");
    };
    reply.as_str().to_owned()
}

fn live_name<S: bevy::state::state::States + core::fmt::Debug>(app: &App) -> Option<String> {
    app.world()
        .get_resource::<State<S>>()
        .map(|state| format!("{:?}", state.get()))
}

#[test]
fn app_phase_reports_every_live_level_from_inside_a_battle() {
    let (mut app, tx) = menu_app_with_net_qa();

    request_battle(&mut app);
    app.update();
    let reached = advance_until(
        &mut app,
        |app| live_name::<BattleScapeState>(app).as_deref() == Some("BattleRunning"),
        DRIVE_BUDGET,
    );
    assert!(
        reached,
        "the fixture must descend to BattleRunning before the phase is worth reading; last \
         observed BattleScapeState was {:?}",
        live_name::<BattleScapeState>(&app),
    );

    let body = read_phase(&mut app, &tx);

    let Some(app_live) = live_name::<AppState>(&app) else {
        unreachable!("`app` is live inside a battle, so its State resource exists");
    };
    assert!(
        body.contains(&format!("app:{app_live}")),
        "`app` must report the app's own live state ({app_live}): {body}",
    );

    for (level, live) in [
        ("running", live_name::<RunningState>(&app)),
        ("game", live_name::<GameState>(&app)),
        ("battlescape", live_name::<BattleScapeState>(&app)),
    ] {
        let Some(live) = live else {
            unreachable!("`{level}` is live inside a battle, so its State resource exists");
        };
        assert!(
            body.contains(&format!("{level}:Some({live})")),
            "`{level}` must report the app's own live state ({live}): {body}",
        );
    }

    assert!(
        body.contains("battlescape:Some(BattleRunning)"),
        "the fixture rests the battle in BattleRunning: {body}",
    );
    assert!(
        body.contains("aftermath:None"),
        "the aftermath has not started, so its level is present and None: {body}",
    );
}
