//! A request paired with a one-shot responder.

use cobalt_mcp_protocol::message::{McpRequest, McpResponse};

use super::Responder;

/// Incoming QA request with a channel to send the response.
pub struct IncomingRequest {
    request:   McpRequest,
    responder: Responder,
}

impl IncomingRequest {
    /// Pair a request with its responder.
    #[must_use]
    pub const fn new(request: McpRequest, responder: Responder) -> Self {
        Self { request, responder }
    }

    /// Borrow the request.
    #[must_use]
    pub const fn request(&self) -> &McpRequest {
        &self.request
    }

    /// Send a response and consume the pair.
    pub fn respond(self, response: McpResponse) {
        self.responder.reply(response);
    }

    /// Split into request and responder.
    #[must_use]
    pub fn into_parts(self) -> (McpRequest, Responder) {
        (self.request, self.responder)
    }
}
