//! The per-connection conversation: read → decode → negotiate or hand to the host → frame
//! the reply back (GTW-736; the handshake gate is GTW-940).

use core::ops::ControlFlow;
use std::{
    io::{self, Read, Write},
    net::TcpStream,
    sync::mpsc::Sender,
};

use gdtf_qa_protocol::{
    framing::{FrameDecoder, encode},
    message::{HelloFacts, QaError, QaRequest, QaResponse},
};

use super::session::{FrameVerdict, SessionState};
use crate::{
    channel::{IncomingRequest, Responder},
    config::NetIoTimeout,
};

/// Serve one client: read frames, negotiate the handshake, correlate every other request with
/// its reply, frame the reply back. Returns (the thread ends) on EOF, a timeout, or any
/// transport error.
///
/// The [`SessionState`] is created here and lives exactly as long as this connection, so a
/// reconnect starts [`Fresh`](SessionState::Fresh) again.
pub(super) fn handle_client(
    mut stream: TcpStream,
    request_tx: &Sender<IncomingRequest>,
    io_timeout: NetIoTimeout,
    facts: &HelloFacts,
) {
    // Two-sided timeouts (bevy-traps: no panic — a failure to set them is non-fatal).
    drop(stream.set_read_timeout(Some(*io_timeout)));
    drop(stream.set_write_timeout(Some(*io_timeout)));
    let mut decoder = FrameDecoder::new();
    let mut session = SessionState::Fresh;
    let mut buf = [0u8; 4096];
    loop {
        let read = match stream.read(&mut buf) {
            // EOF (client closed) or a read timeout / broken socket — reap the client.
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        decoder.push(&buf[..read]);
        loop {
            let frame = match decoder.next_frame() {
                Ok(Some(frame)) => frame,
                Ok(None) => break, // wait for more bytes
                // An oversize prefix desynchronizes the stream irrecoverably — close.
                Err(_) => return,
            };
            if handle_frame(
                &mut stream,
                &frame,
                request_tx,
                io_timeout,
                facts,
                &mut session,
            )
            .is_break()
            {
                return;
            }
        }
    }
}

/// Decode one frame, decide it against the connection's [`SessionState`], and frame the reply
/// back.
///
/// Returns [`ControlFlow::Break`] when the connection must close (the host side is gone or the
/// write failed) and [`ControlFlow::Continue`] otherwise. Two answers never reach the host at
/// all: a frame that does not decode as a [`QaRequest`] is answered
/// [`Malformed`](QaError::Malformed), and any request other than [`Hello`](QaRequest::Hello)
/// on a connection that has not negotiated is answered
/// [`NotNegotiated`](QaError::NotNegotiated). Neither closes the connection.
fn handle_frame(
    stream: &mut TcpStream,
    frame: &gdtf_qa_protocol::framing::Frame,
    request_tx: &Sender<IncomingRequest>,
    io_timeout: NetIoTimeout,
    facts: &HelloFacts,
    session: &mut SessionState,
) -> ControlFlow<()> {
    let Ok(request) = frame.decode::<QaRequest>() else {
        // A DECODE failure: the bytes never became a request, so no host state was consulted
        // and none could be. Advise the client but keep the connection (best-effort write).
        drop(write_frame(stream, &QaResponse::Error(QaError::Malformed)));
        return ControlFlow::Continue(());
    };
    match session.admit(&request, facts) {
        FrameVerdict::Answer(response) => match write_frame(stream, &response) {
            Ok(()) => ControlFlow::Continue(()),
            Err(_) => ControlFlow::Break(()),
        },
        FrameVerdict::Forward => forward_to_host(stream, request, request_tx, io_timeout),
    }
}

/// Hand one negotiated request to the host's inbox and frame the host's reply back.
///
/// Returns [`ControlFlow::Break`] when the host side is gone or the write failed.
fn forward_to_host(
    stream: &mut TcpStream,
    request: QaRequest,
    request_tx: &Sender<IncomingRequest>,
    io_timeout: NetIoTimeout,
) -> ControlFlow<()> {
    let (responder, reply_rx) = Responder::channel();
    if request_tx
        .send(IncomingRequest::new(request, responder))
        .is_err()
    {
        return ControlFlow::Break(());
    }
    // The host's router (or the deadline sweep) answers within a few frames; the socket
    // timeout bounds the wait so a frozen app cannot hang the client forever.
    let response = reply_rx
        .recv_timeout(*io_timeout)
        .unwrap_or(QaResponse::Error(QaError::Timeout));
    match write_frame(stream, &response) {
        Ok(()) => ControlFlow::Continue(()),
        Err(_) => ControlFlow::Break(()),
    }
}

/// Frame `response` (compact RON + `u32` BE length prefix — the protocol crate's codec)
/// and write the whole frame to the socket.
///
/// # Errors
///
/// An [`io::Error`] wrapping a codec failure, or any write error (a closed / timed-out
/// socket).
fn write_frame(stream: &mut TcpStream, response: &QaResponse) -> io::Result<()> {
    let frame = encode(response).map_err(io::Error::other)?;
    stream.write_all(&frame)
}
