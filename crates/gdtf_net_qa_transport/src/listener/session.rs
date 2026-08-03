use gdtf_qa_protocol::message::{HelloFacts, QaError, QaRequest, QaResponse};

/// Whether this connection has negotiated its protocol version yet.
/// The enforcement the handshake lacked: the version was carried, replied to, and then never
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SessionState {
        Fresh,
        Negotiated,
}

pub(super) enum FrameVerdict {
        Answer(QaResponse),
        Forward,
}

impl SessionState {
                                                    pub(super) fn admit(&mut self, request: &QaRequest, facts: &HelloFacts) -> FrameVerdict {
        let QaRequest::Hello(client_version) = request else {
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
