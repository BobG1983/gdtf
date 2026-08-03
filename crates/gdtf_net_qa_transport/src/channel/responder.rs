//! One-shot channel used to reply to an incoming request.

use std::sync::mpsc::{self, Receiver, Sender};

use gdtf_qa_protocol::message::QaResponse;

/// Sends a single QA response back to the listener session.
pub struct Responder(Sender<QaResponse>);

impl Responder {
    /// Create a responder and the matching receiver.
    #[must_use]
    pub fn channel() -> (Self, Receiver<QaResponse>) {
        let (tx, rx) = mpsc::channel();
        (Self(tx), rx)
    }

    /// Send the response (ignores a disconnected receiver).
    pub fn reply(self, response: QaResponse) {
        drop(self.0.send(response));
    }
}
