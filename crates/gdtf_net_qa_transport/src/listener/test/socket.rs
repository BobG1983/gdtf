//! The session suite's socket fixtures: a REAL listener on a REAL loopback socket, and the
//! client-side write / read helpers (GTW-940).
//!
//! Nothing here stands in for the code under test — the listener is
//! [`run_listener`](crate::run_listener) itself, bound by [`bind_listener`](crate::bind_listener)
//! and reached over a real [`TcpStream`], and every frame goes through the protocol crate's real
//! codec. What the fixtures own is only the CLIENT half plus the host inbox receiver, which is
//! what lets a test assert that a frame never reached the host.

use std::{
    error::Error,
    io::{Read, Write},
    net::{Ipv4Addr, TcpStream},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::Duration,
};

use gdtf_qa_protocol::{
    framing::{FrameDecoder, encode},
    message::{HelloFacts, ProtocolVersion, QaRequest, QaResponse, ServerNameNet},
};

use crate::{
    channel::IncomingRequest,
    config::{NetIoTimeout, NetQaPort},
    listener::{bind_listener, run_listener},
};

/// A boxed error so a test's `?` spans both `io::Error` and the codec's `WireError`.
pub(super) type TestResult = Result<(), Box<dyn Error>>;

/// The two-sided socket timeout these tests bind with — generous, because every assertion is
/// driven by a reply arriving rather than by a deadline elapsing.
const TEST_IO_TIMEOUT: NetIoTimeout = NetIoTimeout::new(Duration::from_secs(5));

/// The handshake facts the test host hands the listener.
///
/// The version is DELIBERATELY not [`ProtocolVersion::CURRENT`], and the name is neither
/// host's: a reply carrying these values can only have come from the facts this test passed
/// in, so the tests prove the listener negotiates against the HOST's facts rather than against
/// the protocol crate's constant or a canned literal.
pub(super) fn host_facts() -> HelloFacts {
    HelloFacts::new(
        ProtocolVersion::new(*ProtocolVersion::CURRENT + 41),
        ServerNameNet::new("gdtf-transport-session-test".to_owned()),
    )
}

/// A version the test host does NOT speak — what a stale client sends.
pub(super) fn foreign_version() -> ProtocolVersion {
    ProtocolVersion::CURRENT
}

/// Bind the REAL loopback listener on an OS-assigned ephemeral port, spawn its accept loop
/// with [`host_facts`], connect a client, and hand back the client socket plus the HOST inbox
/// receiver.
///
/// Port `0` yields a fresh port per call, so parallel tests never collide, and the accept-loop
/// `JoinHandle` is discarded exactly as both live hosts discard it.
///
/// # Errors
///
/// Any [`io::Error`](std::io::Error) from the bind, the port readback, or the connect.
pub(super) fn connected_client() -> Result<(TcpStream, Receiver<IncomingRequest>), Box<dyn Error>> {
    let (tx, rx) = mpsc::channel::<IncomingRequest>();
    let (listener, port) = bind_listener(NetQaPort::new(0))?;
    thread::spawn(move || run_listener(listener, tx, TEST_IO_TIMEOUT, host_facts()));
    let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
    // A client-side deadline so a listener that never answers fails the test instead of
    // hanging it.
    stream.set_read_timeout(Some(*TEST_IO_TIMEOUT))?;
    Ok((stream, rx))
}

/// Frame `request` with the real codec and write the whole frame to the socket.
///
/// # Errors
///
/// A codec failure or any write error.
pub(super) fn send(stream: &mut TcpStream, request: &QaRequest) -> TestResult {
    stream.write_all(&encode(request)?)?;
    Ok(())
}

/// Write an already-framed byte sequence — the malformed-payload case, which by definition
/// cannot be produced by encoding a [`QaRequest`].
///
/// # Errors
///
/// Any write error.
pub(super) fn send_raw(stream: &mut TcpStream, frame: &[u8]) -> TestResult {
    stream.write_all(frame)?;
    Ok(())
}

/// Read one whole framed [`QaResponse`] off the socket (tolerating a split read).
///
/// # Errors
///
/// A codec failure, any read error, or the server closing before a whole reply arrived.
pub(super) fn read_response(stream: &mut TcpStream) -> Result<QaResponse, Box<dyn Error>> {
    let mut decoder = FrameDecoder::new();
    let mut buf = [0u8; 512];
    loop {
        if let Some(frame) = decoder.next_frame()? {
            return Ok(frame.decode::<QaResponse>()?);
        }
        let read = stream.read(&mut buf)?;
        if read == 0 {
            return Err("the listener closed before a full response arrived".into());
        }
        decoder.push(&buf[..read]);
    }
}

/// Assert the host inbox holds NOTHING.
///
/// Sound as an immediate check rather than a wait, because the caller has already read the
/// reply for the frame in question: the listener thread writes that reply only AFTER it has
/// decided the frame, so a decision to forward would have put the request in the channel
/// first. `what` names the frame, so a failure says which one leaked.
pub(super) fn assert_inbox_empty(inbox: &Receiver<IncomingRequest>, what: &str) {
    match inbox.try_recv() {
        Err(TryRecvError::Empty) => {}
        Err(TryRecvError::Disconnected) => {
            unreachable!("the listener holds the sender for the whole test, so it is connected")
        }
        Ok(incoming) => unreachable!(
            "{what} must never reach the host inbox, but the host received {:?}",
            incoming.request()
        ),
    }
}
