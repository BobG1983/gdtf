//! The accept loop, its one-client-at-a-time gate, and the `Busy` rejection (GTW-736).

use std::{
    io::Write,
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    thread,
};

use gdtf_qa_protocol::{
    envelope::{HelloFacts, QaError, QaResponse},
    framing::encode,
};

use super::serve::handle_client;
use crate::{channel::IncomingRequest, config::NetIoTimeout};

/// Run the accept loop for the whole process lifetime: serve one client at a time,
/// reject concurrent connections with [`Busy`](QaError::Busy).
///
/// `facts` are the HOST's handshake facts — the protocol version it speaks and the name it
/// identifies itself as. They are taken here, rather than being answered by each host's
/// router, because the listener thread answers every [`Hello`](gdtf_qa_protocol::envelope::QaRequest::Hello)
/// itself (GTW-940): a `Hello` never reaches a host inbox, and every other request is refused
/// [`NotNegotiated`](QaError::NotNegotiated) until one succeeds. A whole [`HelloFacts`] rather
/// than a bare version, because the server NAME is per-host policy (the game's `SERVER_NAME`,
/// the editor's `EDITOR_QA_SERVER_NAME`) — one version parameter could not carry it, and the
/// duplicated `answer_hello` in both hosts' routers would have had to stay.
///
/// A shared `busy` flag admits exactly one active client: the acquiring connection spawns
/// a handler thread that clears the flag on exit; any connection that finds the flag
/// already set is answered [`Busy`](QaError::Busy) and dropped (closed). Each per-client
/// handler thread is likewise detached and self-reaping — it stores `false` into `busy`
/// and returns on EOF / timeout / transport error.
///
/// ## Thread lifetime & shutdown (a DELIBERATE process-lifetime daemon)
///
/// This function is the body of a DETACHED thread: both spawn sites (the host's plugin —
/// `NetQaPlugin::build` in `gdtf_app` — for the live channel, and this crate's own
/// transport-test harness) discard the [`JoinHandle`](std::thread::JoinHandle) on purpose,
/// and the loop blocks on [`incoming`](TcpListener::incoming) with no exit branch — it
/// never returns under normal operation.
///
/// That is intentional, not an oversight: this is a DEV-ONLY channel (the game double-gates
/// its wiring on `cfg(all(debug_assertions, feature = "net_qa"))` and an env var) whose
/// lifetime IS the running app's. There is deliberately NO in-app shutdown / teardown /
/// `AppExit` hook and NO join, because
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
pub fn run_listener(
    listener: TcpListener,
    request_tx: Sender<IncomingRequest>,
    io_timeout: NetIoTimeout,
    facts: HelloFacts,
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
        // One copy per handler thread: the facts are read-only host policy, so each
        // connection answers its own `Hello` without sharing state with any other.
        let facts = facts.clone();
        // Detached per-client handler (JoinHandle discarded deliberately): it is
        // self-reaping — it clears `busy` and returns on EOF / timeout / transport error
        // (`handle_client`), so it needs no join, and it dies with the process alongside
        // the accept loop (see this fn's "Thread lifetime & shutdown" note).
        thread::spawn(move || {
            handle_client(stream, &tx, io_timeout, &facts);
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
