//! Connection-resilience integration — the real [`QaClient`] against a bevy-free stub
//! game that closes connections and is relaunched, proving the client survives both
//! (GTW-755).
//!
//! The game's per-client handler reaps a connection left idle past its socket timeout and
//! a relaunch replaces the process behind the same port. Both leave the client holding a
//! dead connection. These tests stand up a real loopback [`TcpListener`] stub (reusing the
//! protocol crate's REAL framing, never reimplementing it) and drive the real
//! [`QaClient`] through the real [`QaLink`] surface to prove it transparently
//! reconnects instead of surfacing the dead socket as an error.

use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use gdtf_qa_mcp::{QaClient, QaLink, QaPort};
use gdtf_qa_protocol::{
    envelope::{QaError, QaRequest, QaResponse},
    framing::{FrameDecoder, encode},
    view::{AppFlowView, AppStateNet, BattleActiveNet, CaughtUpNet},
};

/// Bind a loopback listener on an OS-assigned port; return it and the bound port number.
fn bind_loopback() -> (TcpListener, u16) {
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) {
        Ok(listener) => listener,
        Err(err) => unreachable!("the test can bind a loopback listener: {err}"),
    };
    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(err) => unreachable!("the listener has a local address: {err}"),
    };
    (listener, port)
}

/// A canned [`QaResponse::AppFlow`] reply — the stub's answer to every request it serves.
const fn app_flow_reply() -> QaResponse {
    QaResponse::AppFlow(AppFlowView::new(
        AppStateNet::Running,
        BattleActiveNet::new(true),
        Vec::new(),
        CaughtUpNet::new(true),
        None,
        None,
    ))
}

/// Read one framed [`QaRequest`] from `stream` and frame an [`app_flow_reply`] back.
/// Returns whether a whole request was served (false on EOF / a broken stream).
fn answer_one(stream: &mut TcpStream) -> bool {
    let mut decoder = FrameDecoder::new();
    let mut buf = [0u8; 1024];
    loop {
        let read = match stream.read(&mut buf) {
            Ok(0) | Err(_) => return false,
            Ok(count) => count,
        };
        decoder.push(&buf[..read]);
        match decoder.next_frame() {
            Ok(Some(_frame)) => {
                let Ok(out) = encode(&app_flow_reply()) else {
                    return false;
                };
                return stream.write_all(&out).is_ok();
            }
            Ok(None) => {}
            Err(_) => return false,
        }
    }
}

/// A `GetAppFlow` request through the real client.
fn get_app_flow(client: &mut QaClient) -> Result<QaResponse, gdtf_qa_mcp::McpError> {
    client.request(QaRequest::GetAppFlow)
}

/// The game reaps a connection between requests (its idle socket timeout): it answers one
/// request per connection, then closes it. The client keeps that connection and reuses it
/// on the next request — writing to a socket the game already closed. The client MUST
/// transparently reconnect so the second request still succeeds (GTW-755 defect 1).
#[test]
fn a_reused_connection_the_game_closed_reconnects_and_succeeds() {
    let (listener, port) = bind_loopback();
    // Serve exactly two connections, ONE request each, closing between — mirroring the
    // game's per-client handler returning (and closing) after an idle read timeout.
    let server = thread::spawn(move || {
        for _ in 0..2 {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            answer_one(&mut stream);
            // `stream` drops here — the connection closes, exactly like the reaped game
            // handler.
        }
    });

    let mut client = QaClient::new(QaPort::new(port));

    let first = get_app_flow(&mut client);
    assert!(
        matches!(first, Ok(QaResponse::AppFlow(_))),
        "the first request opens a fresh connection and succeeds: {first:?}"
    );

    // The stub has now closed connection 1. Today the client reuses the dead connection
    // and this fails with Disconnected / Io; the fix reconnects and it succeeds.
    let second = get_app_flow(&mut client);
    assert!(
        matches!(second, Ok(QaResponse::AppFlow(_))),
        "a request on a connection the game already closed must reconnect and succeed, \
         not surface the dead socket: {second:?}"
    );

    drop(client);
    drop(server.join());
}

/// A relaunch replaces the process behind the same port number. After a launch the tool
/// layer calls [`QaLink::retarget`] with that port; even when the number is unchanged
/// the client MUST drop any existing connection so the next request reaches the NEW
/// process rather than reusing a socket to the old one (GTW-755 defect 2). Proven by
/// counting connections: with the fix, the post-retarget request opens a second
/// connection; without it, the request reuses the first.
#[test]
fn retarget_on_the_same_port_invalidates_the_connection() {
    let (listener, port) = bind_loopback();
    let connections = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&connections);
    // Keep each connection OPEN and serve many requests on it, so a reuse would be served
    // on the SAME connection — the only reason a second connection appears is the client
    // deliberately dropping the first after retarget.
    let server = thread::spawn(move || {
        for _ in 0..2 {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            seen.fetch_add(1, Ordering::SeqCst);
            while answer_one(&mut stream) {}
        }
    });

    let mut client = QaClient::new(QaPort::new(port));

    let first = get_app_flow(&mut client);
    assert!(
        matches!(first, Ok(QaResponse::AppFlow(_))),
        "the first request succeeds on connection 1: {first:?}"
    );
    assert_eq!(
        connections.load(Ordering::SeqCst),
        1,
        "one connection so far"
    );

    // A relaunch on the SAME port: the connection to the (now-replaced) process must be
    // invalidated.
    client.retarget(QaPort::new(port));

    let second = get_app_flow(&mut client);
    assert!(
        matches!(second, Ok(QaResponse::AppFlow(_))),
        "the post-relaunch request succeeds: {second:?}"
    );
    assert_eq!(
        connections.load(Ordering::SeqCst),
        2,
        "retarget must have forced a fresh connection to the relaunched process, not \
         reused the old socket"
    );

    drop(client);
    drop(server.join());
}

/// A genuinely unreachable game (nothing listening) returns an error PROMPTLY and does not
/// retry indefinitely — the reconnect is bounded to a single fresh attempt on a first
/// connection failure (GTW-755 fix clause C3).
#[test]
fn an_unreachable_game_fails_promptly_without_retrying_forever() {
    // Bind then drop, so the port is (almost certainly) free and nothing is listening.
    let (listener, port) = bind_loopback();
    drop(listener);

    let mut client = QaClient::new(QaPort::new(port));
    let started = Instant::now();
    let result = get_app_flow(&mut client);
    let elapsed = started.elapsed();

    assert!(
        result.is_err(),
        "a request to a port with no listener must error, got: {result:?}"
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "the failure must be prompt (no unbounded reconnect loop), took {elapsed:?}"
    );
    // Sanity: a real protocol rejection is NOT what happens here — this is a link failure.
    assert!(
        !matches!(result, Ok(QaResponse::Error(QaError::BadRequest))),
        "an unreachable game is a link error, not a protocol rejection"
    );
}
