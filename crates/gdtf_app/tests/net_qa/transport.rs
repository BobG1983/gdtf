//! Transport tests: the REAL loopback listener over a real [`TcpStream`] (GTW-736).
//!
//! These exercise the TRANSPORT (framing, one-client-at-a-time `Busy`, read-timeout
//! reap), NOT the router — the router is covered by [`super::routing`]. The Bevy side is
//! a minimal stand-in that reads the listener's inbox and replies with a canned
//! `HelloOk`; it is deliberately NOT a copy of the router's dispatch logic, only a
//! collaborator so the transport's request/reply correlation can be observed.
//!
//! The tests return `Result` and use `?` for the socket / codec I/O (the workspace denies
//! `unwrap`/`expect`/`panic!` even in tests); shape checks use `assert!(matches!(…))`.

use std::{
    error::Error,
    io::{Read, Write},
    net::{Ipv4Addr, TcpStream},
    sync::mpsc,
    thread,
    time::Duration,
};

use gdtf_app::test_support::{IncomingRequest, NET_QA_PROTOCOL_VERSION, NetIoTimeout, NetQaPlugin};
use gdtf_qa_protocol::{
    envelope::{HelloFacts, QaError, QaRequest, QaResponse, ServerNameNet},
    framing::{FrameDecoder, encode},
};

/// A boxed error so a test's `?` can span both `io::Error` and the codec `WireError`.
type TestResult = Result<(), Box<dyn Error>>;

/// Spawn a stand-in for the Bevy side: reply to every request with a canned `HelloOk` so
/// the transport's round-trip is observable. NOT the router (see the module doc).
fn spawn_fake_bevy_side(inbox: mpsc::Receiver<IncomingRequest>) {
    thread::spawn(move || {
        while let Ok(incoming) = inbox.recv() {
            let facts = HelloFacts::new(
                NET_QA_PROTOCOL_VERSION,
                ServerNameNet::new("test".to_owned()),
            );
            incoming.respond(QaResponse::HelloOk(facts));
        }
    });
}

/// Frame `request` and write the whole frame to the socket.
fn write_request(stream: &mut TcpStream, request: &QaRequest) -> TestResult {
    let frame = encode(request)?;
    stream.write_all(&frame)?;
    Ok(())
}

/// Read one whole framed [`QaResponse`] off the socket (tolerating a split read).
fn read_response(stream: &mut TcpStream) -> Result<QaResponse, Box<dyn Error>> {
    let mut decoder = FrameDecoder::new();
    let mut buf = [0u8; 512];
    loop {
        if let Some(frame) = decoder.next_frame()? {
            return Ok(frame.decode::<QaResponse>()?);
        }
        let read = stream.read(&mut buf)?;
        assert!(read > 0, "server closed before a full response arrived");
        decoder.push(&buf[..read]);
    }
}

/// A framed Hello round-trips over a real socket, and a SECOND concurrent client — while
/// the first holds the single slot — receives a typed [`Busy`](QaError::Busy) frame.
#[test]
fn hello_round_trips_and_a_second_client_is_busy() -> TestResult {
    // Generous timeout: client A must stay alive (hold the slot) while B connects.
    let (port, inbox) =
        NetQaPlugin::spawn_test_listener(NetIoTimeout::new(Duration::from_secs(5)))?;
    spawn_fake_bevy_side(inbox);

    // Client A: a full round-trip proves the framing + transport AND that A now holds the
    // one client slot (its handler is alive, looping on read).
    let mut client_a = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
    write_request(&mut client_a, &QaRequest::Hello(NET_QA_PROTOCOL_VERSION))?;
    let response_a = read_response(&mut client_a)?;
    assert!(
        matches!(
            &response_a,
            QaResponse::HelloOk(facts) if facts.protocol == NET_QA_PROTOCOL_VERSION
        ),
        "expected HelloOk for client A, got {response_a:?}",
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

/// The two-sided socket timeout reaps an idle client: a client that connects and sends
/// nothing is closed by the server after the read timeout, which the client observes as
/// EOF (a zero-length read).
#[test]
fn an_idle_client_is_reaped_by_the_read_timeout() -> TestResult {
    // Short server timeout so the reap is observed quickly.
    let (port, inbox) =
        NetQaPlugin::spawn_test_listener(NetIoTimeout::new(Duration::from_millis(150)))?;
    spawn_fake_bevy_side(inbox);

    let mut client = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
    // A client-side safety timeout so a broken server fails the test instead of hanging.
    client.set_read_timeout(Some(Duration::from_secs(5)))?;

    // Send nothing: the server's read times out (~150ms), the handler reaps us, and our
    // blocking read returns EOF once the server closes.
    let mut buf = [0u8; 16];
    let read = client.read(&mut buf)?;
    assert_eq!(
        read, 0,
        "the server must close an idle connection via the read-timeout reap",
    );
    Ok(())
}
