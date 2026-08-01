//! GTW-940: the GAME's handshake, over its REAL loopback listener and a REAL socket.
//!
//! The routing suite next door drives the router through an injected inbox, so it can say
//! nothing about the listener the game actually spawns. This file closes that gap the way the
//! editor's `tests/net_qa_hello/` does: [`NetQaPlugin`](gdtf_app::test_support::NetQaPlugin)`::listening`
//! binds the REAL listener on an OS-assigned port, `build` spawns the REAL accept loop through
//! the same `serve` the env-driven arm runs, and the client is a REAL `TcpStream` speaking the
//! REAL framing codec. The fixtures live in [`socket_support`](super::socket_support).
//!
//! What that pins is the half no in-process test could: the facts the GAME hands
//! [`run_listener`](gdtf_net_qa_transport::run_listener). Change that call site to the
//! editor's server name, or to a neighbouring protocol version, and
//! [`hello_over_the_real_listener_answers_the_games_own_facts`] fails — where before it was a
//! line no test reached at all.
//!
//! GTW-942 adds the two GATE cases against this same real listener: bullets 7 and 8 of that
//! ticket's evidence list ask what a REAL GAME does with a pre-handshake `Run` and with a
//! stale version, and the transport's own suite — which drives `run_listener` with fixture
//! facts and a raw channel standing in for a host — cannot answer for the game.

use std::{sync::mpsc, thread};

use gdtf_app::test_support::{NET_QA_PROTOCOL_VERSION, NET_QA_SERVER_NAME};
use gdtf_qa_protocol::{
    command::{CommandArgsJson, CommandName},
    message::{ProtocolVersion, QaError, QaRequest, QaResponse, RunCommand},
};

use super::socket_support::{
    Client, TestError, TestResult, drive_until_reported, game_app_listening,
};

/// The version bullet 8 names: older than the version the game speaks (14 since this ticket
/// widened the envelope), so it is what a client left behind by an envelope change sends.
const STALE_VERSION: ProtocolVersion = ProtocolVersion::new(12);

/// A `Run` — the request bullet 7 sends before any handshake.
fn run_request() -> QaRequest {
    QaRequest::Run(RunCommand::new(
        CommandName::from_static("app.phase"),
        CommandArgsJson::new("{}".to_owned()),
    ))
}

/// The handshake the GAME answers over its own listener carries the GAME's facts — the
/// version it declares it speaks AND its own server name.
///
/// No frame is driven, deliberately: since GTW-940 the listener thread answers `Hello`
/// itself, so a reply that needed the app's drain would be a regression this case would hang
/// on rather than pass.
///
/// The one test that fails if the facts handed to `run_listener` in
/// `crate::dev::net_qa::plugin` are swapped for the editor's name or a neighbouring version.
#[test]
fn hello_over_the_real_listener_answers_the_games_own_facts() -> TestResult {
    let (_app, port) = game_app_listening()?;
    let mut client = Client::connect(port)?;
    let reply = client.exchange(&QaRequest::Hello(NET_QA_PROTOCOL_VERSION))?;
    assert!(
        matches!(
            &reply,
            QaResponse::HelloOk(facts)
                if facts.protocol == NET_QA_PROTOCOL_VERSION
                    && *facts.server == NET_QA_SERVER_NAME
        ),
        "the game's listener must answer HelloOk with the game's OWN facts (version \
         {NET_QA_PROTOCOL_VERSION:?}, server {NET_QA_SERVER_NAME}), so a client can tell which \
         host it reached — got {reply:?}",
    );
    Ok(())
}

/// **GTW-942 bullet 7.** A `Run` sent BEFORE any `Hello` is answered `NotNegotiated` by the
/// real game's listener.
///
/// No frame is driven: the whole claim is that the refusal comes from the listener thread and
/// the request never reaches the app at all, so a reply that needed a drain would mean the
/// gate had leaked. The transport's own suite proves the host CHANNEL stays empty, which only
/// a test holding that channel can see; this one proves the GAME's listener enforces it.
#[test]
fn a_run_before_hello_is_refused_by_the_games_listener() -> TestResult {
    let (_app, port) = game_app_listening()?;
    let mut client = Client::connect(port)?;
    let reply = client.exchange(&run_request())?;
    assert!(
        matches!(reply, QaResponse::Error(QaError::NotNegotiated)),
        "a Run before the handshake must be answered NotNegotiated by the game's own \
         listener, got {reply:?}",
    );
    Ok(())
}

/// **GTW-942 bullet 8.** A `Hello` carrying version 12 — older than the game's own — is
/// answered `VersionMismatch`, and leaves the connection un-negotiated.
///
/// The follow-up `Run` is the load-bearing half: a refused handshake that nonetheless
/// negotiated the connection would let a client speaking a stale envelope carry on and
/// mis-decode every later reply, which is the defect the gate exists to remove.
#[test]
fn a_stale_version_is_refused_and_leaves_the_connection_fresh() -> TestResult {
    assert_ne!(
        STALE_VERSION, NET_QA_PROTOCOL_VERSION,
        "this case is only meaningful while 12 is NOT the version the game speaks",
    );
    let (_app, port) = game_app_listening()?;
    let mut client = Client::connect(port)?;
    let refused = client.exchange(&QaRequest::Hello(STALE_VERSION))?;
    assert!(
        matches!(refused, QaResponse::Error(QaError::VersionMismatch)),
        "a Hello carrying version 12 must be refused VersionMismatch, got {refused:?}",
    );

    let after = client.exchange(&run_request())?;
    assert!(
        matches!(after, QaResponse::Error(QaError::NotNegotiated)),
        "a refused Hello must leave the connection un-negotiated, so the next frame is \
         still NotNegotiated, got {after:?}",
    );
    Ok(())
}

/// Once negotiated, a request crosses the same listener into the game's REAL router and comes
/// back framed — the inbox half of the wiring, over the same socket.
///
/// Both replies are collected on the client thread while the test body drives frames: the
/// handshake costs no frame (the listener thread answers it), but the `Catalogue` reply only
/// exists once the router's drain has run.
#[test]
fn a_negotiated_request_reaches_the_games_real_router() -> TestResult {
    let (mut app, port) = game_app_listening()?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let collected = (|| -> Result<[QaResponse; 2], TestError> {
            let mut client = Client::connect(port)?;
            Ok([
                client.exchange(&QaRequest::Hello(NET_QA_PROTOCOL_VERSION))?,
                client.exchange(&QaRequest::Catalogue)?,
            ])
        })();
        let _sent = tx.send(collected);
    });

    let [hello, catalogue] = drive_until_reported(&mut app, &rx)?;
    assert!(
        matches!(&hello, QaResponse::HelloOk(_)),
        "the connection must negotiate before the measured request goes out, got {hello:?}",
    );
    assert!(
        matches!(&catalogue, QaResponse::Catalogue(published) if !published.entries.is_empty()),
        "a negotiated Catalogue must be answered by the game's real router with the host's \
         own command list, got {catalogue:?}",
    );
    Ok(())
}
