//! The handshake gate over a REAL loopback socket (GTW-940).
//!
//! Every case drives [`run_listener`](crate::run_listener) itself, bound on an ephemeral
//! loopback port, through a real [`TcpStream`](std::net::TcpStream) speaking the protocol
//! crate's real framing codec. The host side is the raw inbox
//! [`Receiver`](std::sync::mpsc::Receiver) rather than either host's router, which is the
//! point: the load-bearing claim of the ticket is that a pre-handshake request never reaches
//! that channel at all, and only a test holding the channel itself can see that.
//!
//! The facts the listener is given ([`host_facts`]) carry a version that is NOT
//! [`ProtocolVersion::CURRENT`](gdtf_qa_protocol::message::ProtocolVersion::CURRENT) and a
//! name neither host uses, so a `HelloOk` carrying them can only have come from the facts
//! passed in — the negotiation is against the HOST's facts, not against a crate constant.

use gdtf_qa_protocol::{
    command::{CommandArgsJson, CommandName},
    framing::encode_frame,
    message::{QaError, QaRequest, QaResponse, RunCommand},
};

use super::socket::{
    TestResult, assert_inbox_empty, connected_client, foreign_version, host_facts, read_response,
    send, send_raw,
};

/// A `Run` — the request the ticket's load-bearing case sends before any handshake.
fn run_request() -> QaRequest {
    QaRequest::Run(RunCommand::new(
        CommandName::from_static("app.phase"),
        CommandArgsJson::new("{}".to_owned()),
    ))
}

/// **Clause 2.** A `Hello` whose version equals the server's is answered `HelloOk` with the
/// host's OWN facts, from the listener thread — the host inbox never sees it.
#[test]
fn a_matching_hello_is_answered_from_the_listener_thread() -> TestResult {
    let (mut client, inbox) = connected_client()?;
    send(&mut client, &QaRequest::Hello(host_facts().protocol))?;
    let reply = read_response(&mut client)?;
    assert!(
        matches!(&reply, QaResponse::HelloOk(facts) if *facts == host_facts()),
        "a matching Hello must be answered with the host's own handshake facts \
         ({:?}), got {reply:?}",
        host_facts(),
    );
    assert_inbox_empty(&inbox, "a Hello");
    Ok(())
}

/// **Clause 2 (the state half).** The successful handshake leaves the connection
/// `Negotiated`: the very next non-`Hello` frame is forwarded instead of refused.
#[test]
fn a_matching_hello_negotiates_the_connection() -> TestResult {
    let (mut client, inbox) = connected_client()?;
    send(&mut client, &QaRequest::Hello(host_facts().protocol))?;
    let hello = read_response(&mut client)?;
    assert!(
        matches!(hello, QaResponse::HelloOk(_)),
        "the handshake must succeed before this case means anything, got {hello:?}",
    );

    send(&mut client, &QaRequest::Catalogue)?;
    let Ok(incoming) = inbox.recv() else {
        unreachable!("a negotiated Catalogue must reach the host inbox");
    };
    assert!(
        matches!(incoming.request(), QaRequest::Catalogue),
        "the host must receive the request that was sent, got {:?}",
        incoming.request(),
    );
    incoming.respond(QaResponse::Error(QaError::Busy));
    let reply = read_response(&mut client)?;
    assert!(
        matches!(reply, QaResponse::Error(QaError::Busy)),
        "the host's reply must come back down the same socket, got {reply:?}",
    );
    Ok(())
}

/// **Clause 3.** A `Hello` whose version differs is answered `VersionMismatch` and the
/// connection stays `Fresh` — proven by the next frame still being refused `NotNegotiated`
/// with nothing reaching the host.
#[test]
fn a_foreign_version_is_refused_and_the_connection_stays_fresh() -> TestResult {
    let (mut client, inbox) = connected_client()?;
    send(&mut client, &QaRequest::Hello(foreign_version()))?;
    let reply = read_response(&mut client)?;
    assert!(
        matches!(reply, QaResponse::Error(QaError::VersionMismatch)),
        "a Hello carrying a version the host does not speak must be refused \
         VersionMismatch, got {reply:?}",
    );
    assert_inbox_empty(&inbox, "a mismatched Hello");

    send(&mut client, &run_request())?;
    let after = read_response(&mut client)?;
    assert!(
        matches!(after, QaResponse::Error(QaError::NotNegotiated)),
        "a refused Hello must leave the connection Fresh, so the next frame is still \
         NotNegotiated, got {after:?}",
    );
    assert_inbox_empty(&inbox, "a Run after a refused Hello");
    Ok(())
}

/// **Clause 4 — the load-bearing case.** A well-formed non-`Hello` frame on a `Fresh`
/// connection is answered `NotNegotiated`, and the host inbox is EMPTY after it: a `Run` sent
/// before `Hello` never reaches the host at all.
#[test]
fn a_run_before_hello_is_refused_and_never_reaches_the_inbox() -> TestResult {
    let (mut client, inbox) = connected_client()?;
    send(&mut client, &run_request())?;
    let reply = read_response(&mut client)?;
    assert!(
        matches!(reply, QaResponse::Error(QaError::NotNegotiated)),
        "a Run before the handshake must be answered NotNegotiated, got {reply:?}",
    );
    assert_inbox_empty(&inbox, "a Run sent before Hello");
    Ok(())
}

/// **Clause 5 (`Fresh`).** A frame that does not decode as a `QaRequest` is answered
/// `Malformed`, and never reaches the host.
#[test]
fn an_undecodable_frame_is_malformed_on_a_fresh_connection() -> TestResult {
    let (mut client, inbox) = connected_client()?;
    send_raw(&mut client, &encode_frame(b"this is not a QaRequest")?)?;
    let reply = read_response(&mut client)?;
    assert!(
        matches!(reply, QaResponse::Error(QaError::Malformed)),
        "a payload that does not decode as a QaRequest must be answered Malformed, \
         got {reply:?}",
    );
    assert_inbox_empty(&inbox, "an undecodable frame");
    Ok(())
}

/// **Clause 5 (`Negotiated`).** The same answer once the connection has negotiated: a decode
/// failure is a decode failure whatever the session state, and it still never reaches the host.
#[test]
fn an_undecodable_frame_is_malformed_on_a_negotiated_connection() -> TestResult {
    let (mut client, inbox) = connected_client()?;
    send(&mut client, &QaRequest::Hello(host_facts().protocol))?;
    let hello = read_response(&mut client)?;
    assert!(
        matches!(hello, QaResponse::HelloOk(_)),
        "the handshake must succeed before this case means anything, got {hello:?}",
    );

    send_raw(&mut client, &encode_frame(b"this is not a QaRequest")?)?;
    let reply = read_response(&mut client)?;
    assert!(
        matches!(reply, QaResponse::Error(QaError::Malformed)),
        "a negotiated connection must answer an undecodable frame Malformed too, \
         got {reply:?}",
    );
    assert_inbox_empty(&inbox, "an undecodable frame on a negotiated connection");
    Ok(())
}

/// **Clause 6.** After a successful `Hello`, `Catalogue` AND `Run` are forwarded to the host
/// inbox and the host's replies come back — the forwarding path is unchanged for a negotiated
/// connection.
#[test]
fn catalogue_and_run_reach_the_inbox_after_a_successful_hello() -> TestResult {
    let (mut client, inbox) = connected_client()?;
    send(&mut client, &QaRequest::Hello(host_facts().protocol))?;
    let hello = read_response(&mut client)?;
    assert!(
        matches!(hello, QaResponse::HelloOk(_)),
        "the handshake must succeed before this case means anything, got {hello:?}",
    );

    for expected in [QaRequest::Catalogue, run_request()] {
        send(&mut client, &expected)?;
        let Ok(incoming) = inbox.recv() else {
            unreachable!("{expected:?} must reach the host inbox once negotiated");
        };
        assert_eq!(
            format!("{:?}", incoming.request()),
            format!("{expected:?}"),
            "the host must receive exactly the request that was sent",
        );
        incoming.respond(QaResponse::Error(QaError::Busy));
        let reply = read_response(&mut client)?;
        assert!(
            matches!(reply, QaResponse::Error(QaError::Busy)),
            "the host's own reply must be framed back to the client, got {reply:?}",
        );
    }
    Ok(())
}
