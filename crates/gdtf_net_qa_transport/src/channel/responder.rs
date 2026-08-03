use std::sync::mpsc::{self, Receiver, Sender};

use gdtf_qa_protocol::message::QaResponse;

pub struct Responder(Sender<QaResponse>);

impl Responder {
                    #[must_use]
    pub fn channel() -> (Self, Receiver<QaResponse>) {
        let (tx, rx) = mpsc::channel();
        (Self(tx), rx)
    }

                    pub fn reply(self, response: QaResponse) {
        drop(self.0.send(response));
    }
}
