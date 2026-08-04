//! A request paired with a one-shot responder.

use gdtf_qa_protocol::message::{QaRequest, QaResponse};

use super::Responder;

/// Incoming QA request with a channel to send the response.
pub struct IncomingRequest {
    request:   QaRequest,
    responder: Responder,
}

impl IncomingRequest {
    /// Pair a request with its responder.
    #[must_use]
    pub const fn new(request: QaRequest, responder: Responder) -> Self {
        Self { request, responder }
    }

    /// Borrow the request.
    #[must_use]
    pub const fn request(&self) -> &QaRequest {
        &self.request
    }

    /// Send a response and consume the pair.
    pub fn respond(self, response: QaResponse) {
        self.responder.reply(response);
    }

    /// Split into request and responder.
    #[must_use]
    pub fn into_parts(self) -> (QaRequest, Responder) {
        (self.request, self.responder)
    }
}
