//! How a case connects, shakes hands, and drives its requests over the real socket.

use bevy::app::App;
use gdtf_app::test_support::NET_QA_PROTOCOL_VERSION;
use gdtf_qa_protocol::{
    command::{CommandArgsRon, CommandName, CommandOutcome, RunOptions, UnavailableCode},
    message::{QaError, QaRequest, QaResponse, RunCommand},
    ports::NetQaPort,
};

use crate::socket_support::{Client, SocketFixture, TestError, battle_app_listening};

/// Build the fixture, let `plan` read the live world to shape the requests, then exchange.
pub(crate) fn exchange_planned(
    fixture: SocketFixture,
    plan: impl FnOnce(&App) -> Vec<QaRequest>,
) -> Result<Vec<QaResponse>, TestError> {
    let (mut app, port) = fixture()?;
    let requests = plan(&app);
    exchange_over(&mut app, port, requests)
}

/// Build the battle fixture, let `prepare` write the world and shape the requests, then exchange.
///
/// The app comes back so a case can read what the sim actually holds after the replies landed.
pub(crate) fn exchange_in_battle(
    prepare: impl FnOnce(&mut App) -> Vec<QaRequest>,
) -> Result<(App, Vec<QaResponse>), TestError> {
    let (mut app, port) = battle_app_listening()?;
    let requests = prepare(&mut app);
    let replies = exchange_over(&mut app, port, requests)?;
    Ok((app, replies))
}

/// Build a fixture that reports what it set up, shape the requests from it, then exchange.
pub(crate) fn exchange_expected<T>(
    fixture: impl FnOnce() -> Result<(App, NetQaPort, T), TestError>,
    plan: impl FnOnce(&T) -> Vec<QaRequest>,
) -> Result<(Vec<QaResponse>, T), TestError> {
    let (_app, replies, expected) = exchange_inspecting(fixture, plan)?;
    Ok((replies, expected))
}

/// Exchange as `exchange_expected` does, and keep the live app so a case can read what it left.
///
/// A reply says what the QA layer answered; the app says what the sim actually holds. A case
/// that has to tell one value from another reads the app.
pub(crate) fn exchange_inspecting<T>(
    fixture: impl FnOnce() -> Result<(App, NetQaPort, T), TestError>,
    plan: impl FnOnce(&T) -> Vec<QaRequest>,
) -> Result<(App, Vec<QaResponse>, T), TestError> {
    let (mut app, port, expected) = fixture()?;
    let requests = plan(&expected);
    let replies = exchange_over(&mut app, port, requests)?;
    Ok((app, replies, expected))
}

/// Exchange `first`, let `between` write the live world, then exchange `second`.
///
/// The world changes with no request in flight, so what each call sees is fixed by the case.
pub(crate) fn exchange_around(
    fixture: SocketFixture,
    first: QaRequest,
    between: impl FnOnce(&mut App),
    second: QaRequest,
) -> Result<(QaResponse, QaResponse), TestError> {
    let (mut app, port) = fixture()?;
    let mut client = greet(&mut app, port)?;
    let before = client.exchange(&mut app, &first)?;
    between(&mut app);
    let after = client.exchange(&mut app, &second)?;
    Ok((before, after))
}

/// Connect and shake hands, so a case can drive its own exchanges one at a time.
pub(crate) fn greet(app: &mut App, port: NetQaPort) -> Result<Client, TestError> {
    let mut client = Client::connect(port)?;
    let hello = client.exchange(app, &QaRequest::Hello(NET_QA_PROTOCOL_VERSION))?;
    if !matches!(hello, QaResponse::HelloOk(_)) {
        return Err(format!("the handshake must succeed first, got {hello:?}").into());
    }
    Ok(client)
}

fn exchange_over(
    app: &mut App,
    port: NetQaPort,
    requests: Vec<QaRequest>,
) -> Result<Vec<QaResponse>, TestError> {
    let mut client = greet(app, port)?;
    let mut replies = Vec::with_capacity(requests.len());
    for request in &requests {
        replies.push(client.exchange(app, request)?);
    }
    Ok(replies)
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

/// Exchange `request` repeatedly until the reply is not `Timeout`.
///
/// A capture answers Timeout while its readback is still in flight; a loaded machine
/// makes that happen more often, never differently.
pub(crate) fn exchange_until_not_timeout(
    fixture: SocketFixture,
    request: QaRequest,
) -> Result<QaResponse, TestError> {
    let (mut app, port) = fixture()?;
    let mut client = greet(&mut app, port)?;
    loop {
        let reply = client.exchange(&mut app, &request)?;
        if !matches!(reply, QaResponse::Error(QaError::Timeout)) {
            return Ok(reply);
        }
    }
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
