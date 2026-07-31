//! A framed handshake + a forwarded round-trip over a real socket, plus the
//! one-client-at-a-time gate (GTW-736; the forwarded leg is GTW-940).

use std::{
    net::{Ipv4Addr, TcpStream},
    time::Duration,
};

use gdtf_net_qa_transport::NetIoTimeout;
use gdtf_qa_protocol::envelope::{ProtocolVersion, QaError, QaRequest, QaResponse};

use super::harness::{
    TestResult, host_reply_facts, read_response, spawn_fake_host_side, spawn_listener, test_facts,
    write_request,
};

/// A framed Hello round-trips over a real socket, a following request is FORWARDED to the
/// stand-in host side and its reply comes back, and a SECOND concurrent client — while the
/// first holds the single slot — receives a typed [`Busy`](QaError::Busy) frame.
#[test]
fn hello_round_trips_and_a_second_client_is_busy() -> TestResult {
    // Generous timeout: client A must stay alive (hold the slot) while B connects.
    let (port, inbox) = spawn_listener(NetIoTimeout::new(Duration::from_secs(5)))?;
    spawn_fake_host_side(inbox);

    // Client A: a full round-trip proves the framing + transport AND that A now holds the
    // one client slot (its handler is alive, looping on read).
    let mut client_a = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
    write_request(&mut client_a, &QaRequest::Hello(ProtocolVersion::CURRENT))?;
    let response_a = read_response(&mut client_a)?;
    assert!(
        matches!(
            &response_a,
            QaResponse::HelloOk(facts) if *facts == test_facts()
        ),
        "expected the LISTENER's own HelloOk for client A, got {response_a:?}",
    );

    // The connection is now negotiated, so the next request is forwarded to the host inbox
    // and correlated with the host's reply — which carries the stand-in host's server name,
    // never the listener's, so the reply can only have come back through the inbox.
    write_request(&mut client_a, &QaRequest::Catalogue)?;
    let forwarded = read_response(&mut client_a)?;
    assert!(
        matches!(
            &forwarded,
            QaResponse::HelloOk(facts) if *facts == host_reply_facts()
        ),
        "expected the forwarded request to be answered by the host side, got {forwarded:?}",
    );

    // Client B connects while A holds the slot -> the listener answers Busy and closes it.
    let mut client_b = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
    let response_b = read_response(&mut client_b)?;
    assert!(
        matches!(&response_b, QaResponse::Error(QaError::Busy)),
        "expected Busy for the second concurrent client, got {response_b:?}",
    );

    // Releasing A frees the slot (its handler reads EOF and clears the flag).
    drop(client_a);
    Ok(())
}
