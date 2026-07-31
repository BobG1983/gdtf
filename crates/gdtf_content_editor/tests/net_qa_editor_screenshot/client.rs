//! The client half — a real socket speaking the real framing codec (GTW-880).

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
    ids::ShotName,
};

use crate::support::{ReplyReport, TestError, TestResult};

/// How long the client waits on its own read before reporting a failure rather than hanging.
const CLIENT_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Read one whole framed [`QaResponse`] off the socket (tolerating a split read).
fn read_response(
    stream: &mut TcpStream,
    decoder: &mut FrameDecoder,
) -> Result<QaResponse, TestError> {
    let mut buf = [0u8; 1024];
    loop {
        if let Some(frame) = decoder.next_frame()? {
            return Ok(frame.decode::<QaResponse>()?);
        }
        let read = stream.read(&mut buf)?;
        if read == 0 {
            return Err("the editor closed before a full response arrived".into());
        }
        decoder.push(&buf[..read]);
    }
}

/// Connect, negotiate, put ONE `TakeScreenshot { name }` on the wire, and forward the single
/// reply.
///
/// The handshake first is required since GTW-940 — the transport refuses every request on a
/// connection that has not negotiated — and it costs the test nothing: the listener thread
/// answers it without the editor running a frame, so `opened` is still signalled before the
/// test body has run one.
///
/// `opened` is signalled the moment the screenshot request is on the wire, before the test body
/// has run a frame — so the test's frame counting starts from a request that is already
/// pending. `reply` carries the one reply the editor sends, so the test body can watch, frame
/// by frame, for the moment it appears.
///
/// # Errors
///
/// Any socket, codec or channel failure, or a handshake the editor refused.
pub(crate) fn request_screenshot(
    port: NetQaPort,
    name: &str,
    opened: &Sender<()>,
    reply: &Sender<ReplyReport>,
) -> TestResult {
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
    stream.set_read_timeout(Some(CLIENT_READ_TIMEOUT))?;
    let mut decoder = FrameDecoder::new();
    stream.write_all(&encode(&QaRequest::Hello(ProtocolVersion::CURRENT))?)?;
    let negotiated = read_response(&mut stream, &mut decoder)?;
    if !matches!(negotiated, QaResponse::HelloOk(_)) {
        return Err(format!("the editor refused the handshake: {negotiated:?}").into());
    }
    let request = QaRequest::TakeScreenshot {
        name: Some(ShotName::new(name.to_owned())),
    };
    stream.write_all(&encode(&request)?)?;
    opened.send(())?;
    let response = read_response(&mut stream, &mut decoder)?;
    reply.send(Ok(response))?;
    Ok(())
}
