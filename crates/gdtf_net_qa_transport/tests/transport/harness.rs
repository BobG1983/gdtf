//! The transport suite's shared fixtures: a real listener, a stand-in host side, and the
//! two socket helpers (GTW-736; lifted in GTW-803).

use std::{
    error::Error,
    io::{Read, Write},
    net::TcpStream,
    sync::mpsc::{self, Receiver},
    thread,
};

use gdtf_net_qa_transport::{
    IncomingRequest, NetIoTimeout, NetQaPort, bind_listener, run_listener,
};
use gdtf_qa_protocol::{
    envelope::{HelloFacts, ProtocolVersion, QaRequest, QaResponse, ServerNameNet},
    framing::{FrameDecoder, encode},
};

/// A boxed error so a test's `?` can span both `io::Error` and the codec `WireError`.
pub(crate) type TestResult = Result<(), Box<dyn Error>>;

/// Bind the REAL loopback listener on an OS-assigned ephemeral port and spawn its accept
/// loop, returning the bound port and the request receiver.
///
/// Binding port `0` yields a fresh port per call, so parallel tests never collide. The
/// accept-loop `JoinHandle` is discarded deliberately, exactly as the live host arm does —
/// the thread runs until the test process exits, which reaps it (see `run_listener`'s
/// "Thread lifetime & shutdown" note).
pub(crate) fn spawn_listener(
    io_timeout: NetIoTimeout,
) -> Result<(NetQaPort, Receiver<IncomingRequest>), Box<dyn Error>> {
    let (tx, rx) = mpsc::channel::<IncomingRequest>();
    let (listener, port) = bind_listener(NetQaPort::new(0))?;
    thread::spawn(move || run_listener(listener, tx, io_timeout));
    Ok((port, rx))
}

/// Spawn a stand-in for the host side: reply to every request with a canned `HelloOk` so
/// the transport's round-trip is observable. NOT a router (see the suite doc).
pub(crate) fn spawn_fake_host_side(inbox: Receiver<IncomingRequest>) {
    thread::spawn(move || {
        while let Ok(incoming) = inbox.recv() {
            let facts = HelloFacts::new(
                ProtocolVersion::CURRENT,
                ServerNameNet::new("test".to_owned()),
            );
            incoming.respond(QaResponse::HelloOk(facts));
        }
    });
}

/// Frame `request` and write the whole frame to the socket.
pub(crate) fn write_request(stream: &mut TcpStream, request: &QaRequest) -> TestResult {
    let frame = encode(request)?;
    stream.write_all(&frame)?;
    Ok(())
}

/// Read one whole framed [`QaResponse`] off the socket (tolerating a split read).
pub(crate) fn read_response(stream: &mut TcpStream) -> Result<QaResponse, Box<dyn Error>> {
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
