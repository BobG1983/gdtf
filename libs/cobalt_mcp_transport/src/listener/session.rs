use cobalt_mcp_protocol::message::{HelloFacts, McpRequest, McpResponse, McpSessionError};

/// Whether this connection has negotiated its protocol version yet.
/// The enforcement the handshake lacked: the version was carried, replied to, and then never
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SessionState {
    Fresh,
    Negotiated,
}

pub(super) enum FrameVerdict {
    Answer(McpResponse),
    Forward,
}

impl SessionState {
    pub(super) fn admit(&mut self, request: &McpRequest, facts: &HelloFacts) -> FrameVerdict {
        let McpRequest::Hello(client_version) = request else {
            return match *self {
                Self::Fresh => {
                    FrameVerdict::Answer(McpResponse::Error(McpSessionError::NotNegotiated))
                }
                Self::Negotiated => FrameVerdict::Forward,
            };
        };
        if *client_version == facts.protocol {
            *self = Self::Negotiated;
            FrameVerdict::Answer(McpResponse::HelloOk(facts.clone()))
        } else {
            FrameVerdict::Answer(McpResponse::Error(McpSessionError::VersionMismatch))
        }
    }
}
