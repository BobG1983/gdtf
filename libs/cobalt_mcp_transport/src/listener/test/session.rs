//! The facts the listener is given ([`host_facts`]) carry a version that is NOT
use cobalt_mcp_protocol::{
    command::{CommandArgsRon, CommandName},
    framing::encode_frame,
    message::{McpRequest, McpResponse, McpSessionError, RunCommand},
};

use super::socket::{
    TestResult, assert_inbox_empty, connected_client, foreign_version, host_facts, read_response,
    send, send_raw,
};

fn run_request() -> McpRequest {
    McpRequest::Run(RunCommand::new(
        CommandName::from_static("app.phase"),
        CommandArgsRon::new("()".to_owned()),
    ))
}

#[test]
fn a_matching_hello_is_answered_from_the_listener_thread() -> TestResult {
    let (mut client, inbox) = connected_client()?;
    send(&mut client, &McpRequest::Hello(host_facts().protocol))?;
    let reply = read_response(&mut client)?;
    assert!(
        matches!(&reply, McpResponse::HelloOk(facts) if *facts == host_facts()),
        "a matching Hello must be answered with the host's own handshake facts \
         ({:?}), got {reply:?}",
        host_facts(),
    );
    assert_inbox_empty(&inbox, "a Hello");
    Ok(())
}

#[test]
fn a_matching_hello_negotiates_the_connection() -> TestResult {
    let (mut client, inbox) = connected_client()?;
    send(&mut client, &McpRequest::Hello(host_facts().protocol))?;
    let hello = read_response(&mut client)?;
    assert!(
        matches!(hello, McpResponse::HelloOk(_)),
        "the handshake must succeed before this case means anything, got {hello:?}",
    );

    send(&mut client, &McpRequest::Catalogue)?;
    let Ok(incoming) = inbox.recv() else {
        unreachable!("a negotiated Catalogue must reach the host inbox");
    };
    assert!(
        matches!(incoming.request(), McpRequest::Catalogue),
        "the host must receive the request that was sent, got {:?}",
        incoming.request(),
    );
    incoming.respond(McpResponse::Error(McpSessionError::Busy));
    let reply = read_response(&mut client)?;
    assert!(
        matches!(reply, McpResponse::Error(McpSessionError::Busy)),
        "the host's reply must come back down the same socket, got {reply:?}",
    );
    Ok(())
}

#[test]
fn a_foreign_version_is_refused_and_the_connection_stays_fresh() -> TestResult {
    let (mut client, inbox) = connected_client()?;
    send(&mut client, &McpRequest::Hello(foreign_version()))?;
    let reply = read_response(&mut client)?;
    assert!(
        matches!(reply, McpResponse::Error(McpSessionError::VersionMismatch)),
        "a Hello carrying a version the host does not speak must be refused \
         VersionMismatch, got {reply:?}",
    );
    assert_inbox_empty(&inbox, "a mismatched Hello");

    send(&mut client, &run_request())?;
    let after = read_response(&mut client)?;
    assert!(
        matches!(after, McpResponse::Error(McpSessionError::NotNegotiated)),
        "a refused Hello must leave the connection Fresh, so the next frame is still \
         NotNegotiated, got {after:?}",
    );
    assert_inbox_empty(&inbox, "a Run after a refused Hello");
    Ok(())
}

#[test]
fn a_run_before_hello_is_refused_and_never_reaches_the_inbox() -> TestResult {
    let (mut client, inbox) = connected_client()?;
    send(&mut client, &run_request())?;
    let reply = read_response(&mut client)?;
    assert!(
        matches!(reply, McpResponse::Error(McpSessionError::NotNegotiated)),
        "a Run before the handshake must be answered NotNegotiated, got {reply:?}",
    );
    assert_inbox_empty(&inbox, "a Run sent before Hello");
    Ok(())
}

#[test]
fn an_undecodable_frame_is_malformed_on_a_fresh_connection() -> TestResult {
    let (mut client, inbox) = connected_client()?;
    send_raw(&mut client, &encode_frame(b"this is not a McpRequest")?)?;
    let reply = read_response(&mut client)?;
    assert!(
        matches!(reply, McpResponse::Error(McpSessionError::Malformed)),
        "a payload that does not decode as a McpRequest must be answered Malformed, \
         got {reply:?}",
    );
    assert_inbox_empty(&inbox, "an undecodable frame");
    Ok(())
}

#[test]
fn an_undecodable_frame_is_malformed_on_a_negotiated_connection() -> TestResult {
    let (mut client, inbox) = connected_client()?;
    send(&mut client, &McpRequest::Hello(host_facts().protocol))?;
    let hello = read_response(&mut client)?;
    assert!(
        matches!(hello, McpResponse::HelloOk(_)),
        "the handshake must succeed before this case means anything, got {hello:?}",
    );

    send_raw(&mut client, &encode_frame(b"this is not a McpRequest")?)?;
    let reply = read_response(&mut client)?;
    assert!(
        matches!(reply, McpResponse::Error(McpSessionError::Malformed)),
        "a negotiated connection must answer an undecodable frame Malformed too, \
         got {reply:?}",
    );
    assert_inbox_empty(&inbox, "an undecodable frame on a negotiated connection");
    Ok(())
}

#[test]
fn catalogue_and_run_reach_the_inbox_after_a_successful_hello() -> TestResult {
    let (mut client, inbox) = connected_client()?;
    send(&mut client, &McpRequest::Hello(host_facts().protocol))?;
    let hello = read_response(&mut client)?;
    assert!(
        matches!(hello, McpResponse::HelloOk(_)),
        "the handshake must succeed before this case means anything, got {hello:?}",
    );

    for expected in [McpRequest::Catalogue, run_request()] {
        send(&mut client, &expected)?;
        let Ok(incoming) = inbox.recv() else {
            unreachable!("{expected:?} must reach the host inbox once negotiated");
        };
        assert_eq!(
            format!("{:?}", incoming.request()),
            format!("{expected:?}"),
            "the host must receive exactly the request that was sent",
        );
        incoming.respond(McpResponse::Error(McpSessionError::Busy));
        let reply = read_response(&mut client)?;
        assert!(
            matches!(reply, McpResponse::Error(McpSessionError::Busy)),
            "the host's own reply must be framed back to the client, got {reply:?}",
        );
    }
    Ok(())
}
