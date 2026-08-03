use gdtf_qa_protocol::message::{QaRequest, QaResponse};

use super::Responder;

pub struct IncomingRequest {
        request:   QaRequest,
        responder: Responder,
}

impl IncomingRequest {
        #[must_use]
    pub const fn new(request: QaRequest, responder: Responder) -> Self {
        Self { request, responder }
    }

        #[must_use]
    pub const fn request(&self) -> &QaRequest {
        &self.request
    }

        pub fn respond(self, response: QaResponse) {
        self.responder.reply(response);
    }

        #[must_use]
    pub fn into_parts(self) -> (QaRequest, Responder) {
        (self.request, self.responder)
    }
}
