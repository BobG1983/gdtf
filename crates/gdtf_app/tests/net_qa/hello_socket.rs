use std::{sync::mpsc, thread};

use gdtf_app::test_support::{NET_QA_PROTOCOL_VERSION, NET_QA_SERVER_NAME};
use gdtf_qa_protocol::{
    command::{CommandArgsRon, CommandName},
    message::{ProtocolVersion, QaError, QaRequest, QaResponse, RunCommand},
};

use super::socket_support::{
    Client, TestError, TestResult, drive_until_reported, game_app_listening,
};

const STALE_VERSION: ProtocolVersion = ProtocolVersion::new(12);

fn run_request() -> QaRequest {
    QaRequest::Run(RunCommand::new(
        CommandName::from_static("app.phase"),
        CommandArgsRon::new("()".to_owned()),
    ))
}

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
