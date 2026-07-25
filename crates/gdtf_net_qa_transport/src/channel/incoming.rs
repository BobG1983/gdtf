//! The decoded-request + responder pair that crosses the listener → host thread
//! boundary (GTW-736).

use gdtf_qa_protocol::envelope::{QaRequest, QaResponse};

use super::Responder;

/// A decoded request plus the [`Responder`] to answer it on — the unit the listener
/// thread hands to the host's [`NetInbox`](super::NetInbox).
///
/// Framework-plumbing struct (typed fields, not a bare-typed domain value): the
/// request vocabulary is [`QaRequest`], the reply channel is [`Responder`].
pub struct IncomingRequest {
    /// The decoded request.
    request:   QaRequest,
    /// The reply channel back to the socket.
    responder: Responder,
}

impl IncomingRequest {
    /// Pair a decoded request with the responder to answer it on.
    #[must_use]
    pub const fn new(request: QaRequest, responder: Responder) -> Self {
        Self { request, responder }
    }

    /// The decoded request — a stand-in host side inspects what arrived.
    #[must_use]
    pub const fn request(&self) -> &QaRequest {
        &self.request
    }

    /// Answer the request, consuming it — a stand-in host side replies through this.
    pub fn respond(self, response: QaResponse) {
        self.responder.reply(response);
    }

    /// Split into the request and its responder — the host router's destructuring point.
    #[must_use]
    pub fn into_parts(self) -> (QaRequest, Responder) {
        (self.request, self.responder)
    }
}
