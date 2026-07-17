//! The request/response channel plumbing between the listener thread and the Bevy
//! router (GTW-736).
//!
//! The listener thread decodes a [`QaRequest`] off the socket and hands it to the Bevy
//! side as an [`IncomingRequest`] carrying a [`Responder`] — the one-shot reply channel
//! back to the socket. The [`NetInbox`] resource holds the receiving end (a
//! [`Mutex`]-wrapped [`std::sync::mpsc::Receiver`], because `Receiver` is `!Sync` and a
//! Bevy [`Resource`] must be `Sync`).

use std::sync::{
    Mutex,
    mpsc::{self, Receiver, Sender},
};

use bevy::prelude::*;
use gdtf_qa_protocol::envelope::{QaRequest, QaResponse};

crate::support_item! {
    /// The one-shot reply channel back to the socket for a single request.
    ///
    /// The listener thread blocks reading the paired [`Receiver`] after handing the
    /// request to the Bevy side; whoever answers — the router directly, a later consumer,
    /// or the deadline sweep — sends the [`QaResponse`] here and the listener frames it
    /// back to the client. Private-inner (no-bare-types); constructed via
    /// [`channel`](Self::channel). Widened to `pub` under `test-support` (the routing test
    /// names it), `pub(crate)` otherwise.
    struct Responder(Sender<QaResponse>);
}

impl Responder {
    crate::support_item! {
        /// Build a responder + the receiving end its reply arrives on.
        ///
        /// The listener holds the [`Receiver`] and blocks on it; the [`Responder`] rides
        /// with the [`IncomingRequest`] to the Bevy side. Widened to `pub` under
        /// `test-support` so the routing test can read the router's reply.
        #[must_use]
        fn channel() -> (Self, Receiver<QaResponse>) {
            let (tx, rx) = mpsc::channel();
            (Self(tx), rx)
        }
    }

    /// Send the reply, consuming the responder so it can answer exactly once.
    ///
    /// A dropped receiver (the client disconnected before the answer was ready) is not
    /// an error worth surfacing — the reply is simply discarded.
    pub(super) fn reply(self, response: QaResponse) {
        // Best-effort: a dropped receiver (client gone) makes this an `Err` we discard.
        drop(self.0.send(response));
    }
}

crate::support_item! {
    /// A decoded request plus the [`Responder`] to answer it on — the unit the listener
    /// thread hands to the Bevy `NetInbox`.
    ///
    /// Framework-plumbing struct (typed fields, not a bare-typed domain value): the
    /// request vocabulary is [`QaRequest`], the reply channel is [`Responder`]. Widened to
    /// `pub` under `test-support` (the integration suite names it), `pub(crate)` otherwise.
    struct IncomingRequest {
        /// The decoded request.
        request:   QaRequest,
        /// The reply channel back to the socket.
        responder: Responder,
    }
}

impl IncomingRequest {
    crate::support_item! {
        /// Pair a decoded request with the responder to answer it on.
        ///
        /// Widened to `pub` under `test-support` so the routing test can enqueue a
        /// request onto the injected inbox exactly as the listener would.
        #[must_use]
        const fn new(request: QaRequest, responder: Responder) -> Self {
            Self { request, responder }
        }
    }

    /// The decoded request — the transport test's stand-in responder inspects what
    /// arrived. Test-only: the binary routes through `into_parts`.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub const fn request(&self) -> &QaRequest {
        &self.request
    }

    /// Answer the request, consuming it — the transport test's stand-in for the Bevy side
    /// replies through this. Test-only: the binary answers via the router / sweep.
    #[cfg(feature = "test-support")]
    pub fn respond(self, response: QaResponse) {
        self.responder.reply(response);
    }

    /// Split into the request and its responder — the router's destructuring seam.
    pub(super) fn into_parts(self) -> (QaRequest, Responder) {
        (self.request, self.responder)
    }
}

/// The Bevy-side inbox: the receiving end of the listener → router request channel.
///
/// Holds a [`Mutex`]-wrapped [`Receiver`] because `std::sync::mpsc::Receiver` is `!Sync`
/// and a Bevy [`Resource`] must be `Sync`; the router locks it briefly each frame to
/// drain whatever the listener pushed. Named-newtype resource with a private inner
/// (no-bare-types framework plumbing).
#[derive(Resource)]
pub(super) struct NetInbox(Mutex<Receiver<IncomingRequest>>);

impl NetInbox {
    /// Wrap the receiving end of the request channel.
    pub(super) const fn new(rx: Receiver<IncomingRequest>) -> Self {
        Self(Mutex::new(rx))
    }

    /// Drain every request the listener has pushed since the last frame.
    ///
    /// Locks the inner [`Mutex`] briefly and non-blockingly pulls until the channel is
    /// empty. A poisoned lock (a listener thread panicked mid-send — it never does) or a
    /// disconnected channel yields nothing rather than panicking (bevy-traps: no panic in
    /// the happy path).
    pub(super) fn drain(&self) -> Vec<IncomingRequest> {
        let Ok(rx) = self.0.lock() else {
            return Vec::new();
        };
        rx.try_iter().collect()
    }
}
