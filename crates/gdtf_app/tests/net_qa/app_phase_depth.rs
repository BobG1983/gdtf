//! GTW-942: `app.phase` reports the DEEP levels of the state machine, from a real battle.
//!
//! The bullet-3 case in [`commands`](super::commands) answers from the menu, where only the
//! top two levels are live and the other three are legitimately `null`. That leaves the
//! reason `app.phase` was chosen as the first command — "it closes the gap where the four
//! sub-states are entirely absent today" — unproven for three of the four: `GameFactsParam`
//! could hand back `None` for every nested level and the menu case would still pass.
//!
//! So this suite carries the app all the way into a battle through the REAL menu → battle
//! descent ([`battle_fixture`](super::battle_fixture)'s app and the menu's own
//! `StartBattleRequested`), then asks over the REAL router and asserts all five levels by
//! VALUE against the live `State<…>` resources the same app holds.

use std::sync::mpsc;

use bevy::{app::App, state::state::State};
use gdtf_app::test_support::{AppState, BattleScapeState, GameState, RunningState};
use gdtf_net_qa_transport::{IncomingRequest, Responder};
use gdtf_qa_protocol::{
    command::{CommandArgsJson, CommandName, CommandOutcome, RunOptions},
    message::{QaRequest, QaResponse, RunCommand},
};
use gdtf_test_utils::advance_until;
use serde_json::Value;

use super::{
    battle_fixture::{DRIVE_BUDGET, menu_app_with_net_qa, request_battle},
    command_exchange::APP_PHASE,
};

/// Push a request onto the router's inbox and return the channel its reply arrives on.
fn send(tx: &mpsc::Sender<IncomingRequest>, request: QaRequest) -> mpsc::Receiver<QaResponse> {
    let (responder, reply_rx) = Responder::channel();
    let sent = tx.send(IncomingRequest::new(request, responder));
    assert!(sent.is_ok(), "the router inbox must be open");
    reply_rx
}

/// Run `app.phase` over the real router and hand back its reply body as JSON.
fn read_phase(app: &mut App, tx: &mpsc::Sender<IncomingRequest>) -> Value {
    let reply = send(
        tx,
        QaRequest::Run(RunCommand::with_options(
            CommandName::from_static(APP_PHASE),
            CommandArgsJson::new("{}".to_owned()),
            RunOptions::default(),
        )),
    );
    // One frame for the router to admit and park the call, one for the handler to answer it.
    app.update();
    app.update();
    let Ok(QaResponse::Outcome(CommandOutcome::Ran { reply, .. })) = reply.try_recv() else {
        unreachable!("a plain app.phase call must RUN");
    };
    let Ok(body) = serde_json::from_str::<Value>(reply.as_str()) else {
        unreachable!(
            "the reply body is the command's own JSON: {}",
            reply.as_str()
        );
    };
    body
}

/// Read a state resource's `Debug` name, or `None` when that level is not live.
fn live_name<S: bevy::state::state::States + core::fmt::Debug>(app: &App) -> Option<String> {
    app.world()
        .get_resource::<State<S>>()
        .map(|state| format!("{:?}", state.get()))
}

/// In a live battle, all four nested levels are reported, and each matches the app's own
/// `State<…>` resource.
///
/// Comparing against the LIVE resources rather than literals is what makes this fail if
/// `GameFactsParam` drops a level (it would report `null` where the resource exists) or if a
/// wire mirror renames one (the two names would disagree). The menu case cannot see either.
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
    let phase = &body["phase"];

    // The three levels that are live inside a battle, each against the app's own resource.
    for (level, live) in [
        ("app", live_name::<AppState>(&app)),
        ("running", live_name::<RunningState>(&app)),
        ("game", live_name::<GameState>(&app)),
        ("battlescape", live_name::<BattleScapeState>(&app)),
    ] {
        let Some(live) = live else {
            unreachable!("`{level}` is live inside a battle, so its State resource exists");
        };
        assert_eq!(
            phase[level].as_str(),
            Some(live.as_str()),
            "`{level}` must report the app's own live state ({live}): {body}",
        );
    }

    // The battle is running, not in its aftermath, so that one level really is absent — and
    // absent is an explicit null, never a missing key.
    assert_eq!(
        phase["battlescape"], "BattleRunning",
        "the fixture rests the battle in BattleRunning: {body}",
    );
    assert_eq!(
        phase.get("aftermath"),
        Some(&Value::Null),
        "the aftermath has not started, so its level is present and null: {body}",
    );
}
