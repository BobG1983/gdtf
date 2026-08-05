use std::{sync::mpsc, thread};

use bevy::app::App;
use gdtf_app::test_support::NET_QA_PROTOCOL_VERSION;
use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::{
    command::{CommandArgsRon, CommandName, CommandOutcome, RunOptions, UnavailableCode},
    message::{QaRequest, QaResponse, RunCommand},
};

use super::socket_support::{Client, SocketFixture, TestError, drive_until_reported};

pub(crate) const APP_PHASE: &str = "app.phase";

pub(crate) const CAPTURE_SCREENSHOT: &str = "capture.screenshot";

pub(crate) const SETTINGS_READ: &str = "settings.read";

pub(crate) const UI_FOCUS: &str = "ui.focus";

pub(crate) const PLAYBACK_STATE: &str = "playback.state";

pub(crate) const BATTLE_ROSTER: &str = "battle.roster";

pub(crate) const BATTLE_TURN: &str = "battle.turn";

pub(crate) const BATTLE_SELECTION: &str = "battle.selection";

pub(crate) const BATTLE_OFFERS: &str = "battle.offers";

pub(crate) const BATTLE_INSPECT: &str = "battle.inspect";

pub(crate) const BATTLE_SIGHTLINE: &str = "battle.sightline";

pub(crate) const BATTLE_VISIBLE: &str = "battle.visible";

pub(crate) const LOG_READ: &str = "log.read";

pub(crate) const BATTLE_START: &str = "battle.start";

pub(crate) const BATTLE_FLEE: &str = "battle.flee";

pub(crate) const PROCGEN_STEP: &str = "procgen.step";

pub(crate) const WAIT: &str = "wait";

/// Build the fixture, let `plan` read the live world to shape the requests, then exchange.
pub(crate) fn exchange_planned(
    fixture: SocketFixture,
    plan: impl FnOnce(&App) -> Vec<QaRequest>,
) -> Result<Vec<QaResponse>, TestError> {
    let (mut app, port) = fixture()?;
    let requests = plan(&app);
    exchange_over(&mut app, port, requests)
}

/// Build a fixture that reports what it set up, shape the requests from it, then exchange.
pub(crate) fn exchange_expected<T>(
    fixture: impl FnOnce() -> Result<(App, NetQaPort, T), TestError>,
    plan: impl FnOnce(&T) -> Vec<QaRequest>,
) -> Result<(Vec<QaResponse>, T), TestError> {
    let (mut app, port, expected) = fixture()?;
    let requests = plan(&expected);
    let replies = exchange_over(&mut app, port, requests)?;
    Ok((replies, expected))
}

fn exchange_over(
    app: &mut App,
    port: NetQaPort,
    requests: Vec<QaRequest>,
) -> Result<Vec<QaResponse>, TestError> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let collected = (|| -> Result<Vec<QaResponse>, TestError> {
            let mut client = Client::connect(port)?;
            let hello = client.exchange(&QaRequest::Hello(NET_QA_PROTOCOL_VERSION))?;
            if !matches!(hello, QaResponse::HelloOk(_)) {
                return Err(format!("the handshake must succeed first, got {hello:?}").into());
            }
            let mut replies = Vec::with_capacity(requests.len());
            for request in &requests {
                replies.push(client.exchange(request)?);
            }
            Ok(replies)
        })();
        let _sent = tx.send(collected);
    });
    drive_until_reported(app, &rx)
}

pub(crate) fn exchange_all(
    fixture: SocketFixture,
    requests: Vec<QaRequest>,
) -> Result<Vec<QaResponse>, TestError> {
    exchange_planned(fixture, move |_app| requests)
}

pub(crate) fn exchange(
    fixture: SocketFixture,
    request: QaRequest,
) -> Result<QaResponse, TestError> {
    let mut replies = exchange_all(fixture, vec![request])?;
    replies.pop().ok_or_else(|| "no reply arrived".into())
}

pub(crate) fn run(name: &'static str, arguments: &str, options: RunOptions) -> QaRequest {
    QaRequest::Run(RunCommand::with_options(
        CommandName::from_static(name),
        CommandArgsRon::new(arguments.to_owned()),
        options,
    ))
}

/// The RON body of a reply that ran, or a failure naming what came back instead.
pub(crate) fn ran_body(name: &str, reply: QaResponse) -> Result<String, TestError> {
    match reply {
        QaResponse::Outcome(CommandOutcome::Ran { reply, .. }) => Ok(reply.as_str().to_owned()),
        other => Err(format!("`{name}` must RUN in this fixture, got {other:?}").into()),
    }
}

/// Assert a command refuses with `WrongState` on a host that is not in a battle.
pub(crate) fn assert_refused_off_the_battle_screen(
    fixture: SocketFixture,
    name: &'static str,
    arguments: &str,
) -> Result<(), TestError> {
    let reply = exchange(fixture, run(name, arguments, RunOptions::default()))?;
    let QaResponse::Outcome(CommandOutcome::Unavailable { code, note }) = reply else {
        return Err(format!("`{name}` must refuse outside a battle, got {reply:?}").into());
    };
    if code != UnavailableCode::WrongState {
        return Err(
            format!("`{name}` must refuse with WrongState, got {code:?} — {note:?}").into(),
        );
    }
    Ok(())
}
