//! The loopback TCP transport — the `std::thread` listener (GTW-736).
//!
//! Binds [`Ipv4Addr::LOCALHOST`] ONLY (never a routable interface, never configurable —
//! only the port varies) and serves ONE client at a time: a second concurrent connection
//! receives a typed [`Busy`](QaError::Busy) frame and is closed. The client socket gets
//! two-sided read AND write timeouts so a stuck or idle client cannot hold the single
//! slot forever. Frames follow the crate framing module (`u32` BE length prefix + compact
//! RON) — this transport calls the crate's pure codec, it never reimplements framing.

use core::ops::ControlFlow;
use std::{
    io::{self, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    thread,
};

use gdtf_qa_protocol::{
    envelope::{QaError, QaRequest, QaResponse},
    framing::{FrameDecoder, encode},
};

use super::{
    channel::{IncomingRequest, Responder},
    config::{NetIoTimeout, NetQaPort},
};

/// Bind the loopback listener on `port`, returning it and the ACTUAL port bound.
///
/// The interface is ALWAYS [`Ipv4Addr::LOCALHOST`]. A `port` of `0` asks the OS for a
/// free ephemeral port, which [`local_addr`](TcpListener::local_addr) reads back — the
/// deterministic-test recipe (no fixed port to collide on).
///
/// # Errors
///
/// Any [`io::Error`] from [`TcpListener::bind`] / [`TcpListener::local_addr`] (e.g. the
/// requested port already in use).
pub(super) fn bind_listener(port: NetQaPort) -> io::Result<(TcpListener, NetQaPort)> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, *port))?;
    let actual = NetQaPort::new(listener.local_addr()?.port());
    Ok((listener, actual))
}

/// Run the accept loop for the whole process lifetime: serve one client at a time,
/// reject concurrent connections with [`Busy`](QaError::Busy).
///
/// A shared `busy` flag admits exactly one active client: the acquiring connection spawns
/// a handler thread that clears the flag on exit; any connection that finds the flag
/// already set is answered [`Busy`](QaError::Busy) and dropped (closed). Each per-client
/// handler thread is likewise detached and self-reaping — it stores `false` into `busy`
/// and returns on EOF / timeout / transport error (see [`handle_client`]).
///
/// ## Thread lifetime & shutdown (a DELIBERATE process-lifetime daemon)
///
/// This function is the body of a DETACHED thread: both spawn sites
/// ([`NetQaPlugin::build`](super::plugin) for the live channel and
/// [`spawn_test_listener`](super::plugin) for the transport test) discard the
/// [`JoinHandle`](std::thread::JoinHandle) on purpose, and the loop blocks on
/// [`incoming`](TcpListener::incoming) with no exit branch — it never returns under normal
/// operation.
///
/// That is intentional, not an oversight: `net_qa` is a DEV-ONLY channel (double-gated
/// `cfg(all(debug_assertions, feature = "net_qa"))` and env-gated) whose lifetime IS the
/// running app's. There is deliberately NO in-app shutdown / teardown / `AppExit` hook and
/// NO join, because
///
/// - a QA control channel that outlived the process it controls would be meaningless — the
///   channel must stay open for the entire dev session, so killing the process IS the
///   shutdown, and the OS reaps the thread and closes the listening socket on process exit;
/// - a thread parked in a blocking `accept` cannot be cancelled from safe `std` without a
///   self-connect / shutdown-socket wakeup, which would be dead weight for a tool whose
///   only correct lifetime is "as long as the app runs".
///
/// So the discarded handle leaks nothing that survives the process, and no code depends on
/// joining it. (A future graceful in-session teardown, if ever wanted, is out of this
/// ticket's scope — GTW-736 specifies bind-and-serve, not a stop control.)
pub(super) fn run_listener(
    listener: TcpListener,
    request_tx: Sender<IncomingRequest>,
    io_timeout: NetIoTimeout,
) {
    let busy = Arc::new(AtomicBool::new(false));
    for stream in listener.incoming() {
        let Ok(stream) = stream else {
            continue;
        };
        if busy.swap(true, Ordering::AcqRel) {
            // Already serving a client — reject this concurrent connection and do NOT
            // clear the flag (we did not acquire the slot).
            reject_busy(stream);
            continue;
        }
        let tx = request_tx.clone();
        let busy = Arc::clone(&busy);
        // Detached per-client handler (JoinHandle discarded deliberately): it is
        // self-reaping — it clears `busy` and returns on EOF / timeout / transport error
        // (`handle_client`), so it needs no join, and it dies with the process alongside
        // the accept loop (see this fn's "Thread lifetime & shutdown" note).
        thread::spawn(move || {
            handle_client(stream, &tx, io_timeout);
            busy.store(false, Ordering::Release);
        });
    }
}

/// Best-effort: frame a [`Busy`](QaError::Busy) error to the rejected client, then close
/// (the stream drops at the end of scope).
fn reject_busy(mut stream: TcpStream) {
    if let Ok(frame) = encode(&QaResponse::Error(QaError::Busy)) {
        // Best-effort — the rejected client may already be gone.
        drop(stream.write_all(&frame));
    }
}

/// Serve one client: read frames, correlate each request with its reply, frame the reply
/// back. Returns (the thread ends) on EOF, a timeout, or any transport error.
fn handle_client(
    mut stream: TcpStream,
    request_tx: &Sender<IncomingRequest>,
    io_timeout: NetIoTimeout,
) {
    // Two-sided timeouts (bevy-traps: no panic — a failure to set them is non-fatal).
    drop(stream.set_read_timeout(Some(*io_timeout)));
    drop(stream.set_write_timeout(Some(*io_timeout)));
    let mut decoder = FrameDecoder::new();
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
            if handle_frame(&mut stream, &frame, request_tx, io_timeout).is_break() {
                return;
            }
        }
    }
}

/// Decode one frame, route it to the Bevy side, and frame the reply back.
///
/// Returns [`ControlFlow::Break`] when the connection must close (the Bevy side is gone
/// or the write failed) and [`ControlFlow::Continue`] otherwise. A malformed payload is
/// answered [`BadRequest`](QaError::BadRequest) without closing the connection.
fn handle_frame(
    stream: &mut TcpStream,
    frame: &gdtf_qa_protocol::framing::Frame,
    request_tx: &Sender<IncomingRequest>,
    io_timeout: NetIoTimeout,
) -> ControlFlow<()> {
    let Ok(request) = frame.decode::<QaRequest>() else {
        // Advise the client but keep the connection (best-effort write).
        drop(write_frame(stream, &QaResponse::Error(QaError::BadRequest)));
        return ControlFlow::Continue(());
    };
    let (responder, reply_rx) = Responder::channel();
    if request_tx
        .send(IncomingRequest::new(request, responder))
        .is_err()
    {
        return ControlFlow::Break(());
    }
    // The Bevy router (or the deadline sweep) answers within a few frames; the socket
    // timeout bounds the wait so a frozen app cannot hang the client forever.
    let response = reply_rx
        .recv_timeout(*io_timeout)
        .unwrap_or(QaResponse::Error(QaError::Timeout));
    match write_frame(stream, &response) {
        Ok(()) => ControlFlow::Continue(()),
        Err(_) => ControlFlow::Break(()),
    }
}

/// Frame `response` (compact RON + `u32` BE length prefix — the crate codec) and write
/// the whole frame to the socket.
///
/// # Errors
///
/// An [`io::Error`] wrapping a codec failure, or any write error (a closed / timed-out
/// socket).
fn write_frame(stream: &mut TcpStream, response: &QaResponse) -> io::Result<()> {
    let frame = encode(response).map_err(io::Error::other)?;
    stream.write_all(&frame)
}

#[cfg(test)]
mod test {
    use super::{NetQaPort, bind_listener};

    /// Binding port `0` yields a real, non-zero OS-assigned ephemeral port.
    #[test]
    fn bind_zero_resolves_to_a_real_port() {
        let bound = bind_listener(NetQaPort::new(0));
        assert!(
            bound.is_ok(),
            "binding loopback:0 must succeed, got {bound:?}"
        );
        let Ok((listener, port)) = bound else {
            return;
        };
        assert!(*port > 0, "OS should assign a real ephemeral port");
        drop(listener);
    }
}
