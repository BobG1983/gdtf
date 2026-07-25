//! The one-shot reply channel back to the socket for a single request (GTW-736).

use std::sync::mpsc::{self, Receiver, Sender};

use gdtf_qa_protocol::envelope::QaResponse;

/// The one-shot reply channel back to the socket for a single request.
///
/// The listener thread blocks reading the paired [`Receiver`] after handing the
/// request to the host side; whoever answers — the host's router, a later consumer,
/// or the deadline sweep — sends the [`QaResponse`] here and the listener frames it
/// back to the client. Private-inner (no-bare-types); constructed via
/// [`channel`](Self::channel).
pub struct Responder(Sender<QaResponse>);

impl Responder {
    /// Build a responder + the receiving end its reply arrives on.
    ///
    /// The listener holds the [`Receiver`] and blocks on it; the [`Responder`] rides
    /// with the [`IncomingRequest`](super::IncomingRequest) to the host side.
    #[must_use]
    pub fn channel() -> (Self, Receiver<QaResponse>) {
        let (tx, rx) = mpsc::channel();
        (Self(tx), rx)
    }

    /// Send the reply, consuming the responder so it can answer exactly once.
    ///
    /// A dropped receiver (the client disconnected before the answer was ready) is not
    /// an error worth surfacing — the reply is simply discarded.
    pub fn reply(self, response: QaResponse) {
        // Best-effort: a dropped receiver (client gone) makes this an `Err` we discard.
        drop(self.0.send(response));
    }
}
