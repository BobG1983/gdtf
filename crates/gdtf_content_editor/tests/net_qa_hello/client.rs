//! The client half — a real socket speaking the real framing codec (GTW-804; split out of the
//! flat file in GTW-896, which added the `Load`-phase half).

use std::{
    io::{Read as _, Write as _},
    net::{Ipv4Addr, TcpStream},
    sync::mpsc::Sender,
    time::Duration,
};

use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::{
    envelope::{ProtocolVersion, QaRequest, QaResponse},
    framing::{FrameDecoder, encode},
};

use crate::support::{ClientResult, TestError};

/// How long the client waits on its own read before reporting a failure instead of hanging.
const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// A connected client socket with its own decoder.
///
/// ONE decoder across the whole connection: a reply may arrive in the same read as the tail of
/// the previous one, and the decoder owns that leftover.
struct Client {
    /// The real socket to the editor's real listener.
    stream:  TcpStream,
    /// The real framing decoder the replies are read through.
    decoder: FrameDecoder,
}

impl Client {
    /// Connect to the editor's real listener on loopback.
    fn connect(port: NetQaPort) -> Result<Self, TestError> {
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
        stream.set_read_timeout(Some(READ_TIMEOUT))?;
        Ok(Self {
            stream,
            decoder: FrameDecoder::new(),
        })
    }

    /// Put one request on the wire without waiting for its reply.
    fn send(&mut self, request: &QaRequest) -> Result<(), TestError> {
        self.stream.write_all(&encode(request)?)?;
        Ok(())
    }

    /// Read one whole framed [`QaResponse`] off the socket (tolerating a split read).
    fn read(&mut self) -> Result<QaResponse, TestError> {
        let mut buf = [0u8; 512];
        loop {
            if let Some(frame) = self.decoder.next_frame()? {
                return Ok(frame.decode::<QaResponse>()?);
            }
            let read = self.stream.read(&mut buf)?;
            assert!(read > 0, "the editor closed before a full response arrived");
            self.decoder.push(&buf[..read]);
        }
    }

    /// Send one request and read its reply.
    fn exchange(&mut self, request: &QaRequest) -> Result<QaResponse, TestError> {
        self.send(request)?;
        self.read()
    }
}

/// A version one above the one both hosts speak — the value a mismatched client sends.
pub(crate) fn wrong_version() -> ProtocolVersion {
    ProtocolVersion::new(*ProtocolVersion::CURRENT + 1)
}

/// The `Editing`-side client half: the three exchanges over ONE connection, in order, reported
/// as one batch.
///
/// The transport serves one client at a time, so a single connection also avoids racing the
/// slot's release.
///
/// # Errors
///
/// Any socket or codec failure.
pub(crate) fn exchange_while_editing(port: NetQaPort) -> ClientResult {
    let mut client = Client::connect(port)?;
    Ok(vec![
        client.exchange(&QaRequest::Hello(ProtocolVersion::CURRENT))?,
        client.exchange(&QaRequest::Hello(wrong_version()))?,
        client.exchange(&QaRequest::GetAppFlow)?,
    ])
}

/// The `Load`-side client half (GTW-896): ONE request, put on the wire before the app has run a
/// single frame.
///
/// `opened` is signalled the moment that request is on the wire, so the test body can withhold
/// its first frame until then. That ordering is what makes the `Load` observation deterministic
/// rather than a race with the asset pass: the request is already pending when the editor's
/// first `Update` runs its drain, while the editor's own `Load` → `Editing` gate needs seven
/// frames to resolve the content folders `Startup` has only just asked for.
///
/// ONE request per connection, and one connection per test, is what keeps that margin: every
/// further exchange on the same connection would cost another frame, because the transport is
/// lockstep per client (`handle_frame` in `crates/gdtf_net_qa_transport/src/listener/serve.rs`
/// blocks on request N's reply before it decodes request N + 1).
///
/// # Errors
///
/// Any socket, codec or channel failure.
pub(crate) fn exchange_during_load(
    port: NetQaPort,
    opened: &Sender<()>,
    request: QaRequest,
) -> Result<QaResponse, TestError> {
    let mut client = Client::connect(port)?;
    client.send(&request)?;
    opened.send(())?;
    client.read()
}
