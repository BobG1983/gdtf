//! One-shot channel used to reply to an incoming request.

use std::sync::mpsc::{self, Receiver, Sender};

use cobalt_mcp_protocol::message::McpResponse;

/// Sends a single QA response back to the listener session.
pub struct Responder(Sender<McpResponse>);

impl Responder {
    /// Create a responder and the matching receiver.
    #[must_use]
    pub fn channel() -> (Self, Receiver<McpResponse>) {
        let (tx, rx) = mpsc::channel();
        (Self(tx), rx)
    }

    /// Send the response (ignores a disconnected receiver).
    pub fn reply(self, response: McpResponse) {
        drop(self.0.send(response));
    }
}
