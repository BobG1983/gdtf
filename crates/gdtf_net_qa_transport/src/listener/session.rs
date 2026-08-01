//! The per-connection handshake state and the pre-handshake gate (GTW-940).

use gdtf_qa_protocol::message::{HelloFacts, QaError, QaRequest, QaResponse};

/// Whether this connection has negotiated its protocol version yet.
///
/// The enforcement the handshake lacked: the version was carried, replied to, and then never
/// consulted again, so a client speaking a stale envelope got a `HelloOk` and silently
/// mis-decoded every later reply. A connection that has not negotiated is now answered
/// [`NotNegotiated`](QaError::NotNegotiated) for every non-[`Hello`](QaRequest::Hello) frame,
/// in the listener thread, before the host's inbox ever sees it.
///
/// One value per connection, owned by `handle_client` — a second client's state can never be
/// read or written by the first, because the value never leaves that connection's stack frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SessionState {
    /// No successful `Hello` yet — only `Hello` is serviced.
    Fresh,
    /// The version matched; every request is forwarded to the host.
    Negotiated,
}

/// What the listener thread does with one decoded request.
///
/// Returned by [`SessionState::admit`] so the decision and the socket write stay in separate
/// files: this module decides, `serve` writes.
pub(super) enum FrameVerdict {
    /// Answer from the listener thread, without touching the host inbox.
    Answer(QaResponse),
    /// Hand the request to the host's inbox and wait for its reply.
    Forward,
}

impl SessionState {
    /// Decide what happens to `request` on this connection, advancing the state when a
    /// [`Hello`](QaRequest::Hello) negotiates.
    ///
    /// `facts` are the HOST's handshake facts — the version it speaks and the name it
    /// identifies itself as — passed down from
    /// [`run_listener`](super::run_listener), so the two hosts answer with their own
    /// identities from one implementation.
    ///
    /// A `Hello` whose version matches is answered [`HelloOk`](QaResponse::HelloOk) and moves
    /// the connection to [`Negotiated`](Self::Negotiated); one that differs is answered
    /// [`VersionMismatch`](QaError::VersionMismatch) and leaves the state alone (a `Fresh`
    /// connection stays `Fresh`). Anything else is forwarded only once negotiated.
    pub(super) fn admit(&mut self, request: &QaRequest, facts: &HelloFacts) -> FrameVerdict {
        let QaRequest::Hello(client_version) = request else {
            // EVERY other request kind, present and future, is gated: a new variant is
            // refused before the handshake by construction rather than by remembering to
            // list it here.
            return match *self {
                Self::Fresh => FrameVerdict::Answer(QaResponse::Error(QaError::NotNegotiated)),
                Self::Negotiated => FrameVerdict::Forward,
            };
        };
        if *client_version == facts.protocol {
            *self = Self::Negotiated;
            FrameVerdict::Answer(QaResponse::HelloOk(facts.clone()))
        } else {
            FrameVerdict::Answer(QaResponse::Error(QaError::VersionMismatch))
        }
    }
}
