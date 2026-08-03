use core::fmt::{self, Display};
use std::io;

use gdtf_qa_protocol::{framing::WireError, message::QaError};

#[derive(Debug)]
pub enum McpError {
            Connect(io::Error),
        Io(io::Error),
        Wire(WireError),
        Disconnected,
                                    Handshake(QaError),
            UnexpectedResponse,
}

impl Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect(err) => write!(
                f,
                "could not connect to the game's net_qa channel (is the game running \
                 with GDTF_NET_QA=1?): {err}"
            ),
            Self::Io(err) => write!(f, "net_qa connection I/O failed: {err}"),
            Self::Wire(err) => write!(f, "net_qa framing failed: {err}"),
            Self::Disconnected => f.write_str("the game closed the net_qa connection"),
            Self::Handshake(err) => write!(
                f,
                "the net_qa handshake was refused ({err:?}) — the child speaks a different \
                 protocol version than this build of the MCP bridge; rebuild it from the same \
                 tree"
            ),
            Self::UnexpectedResponse => {
                f.write_str("the game answered with an unexpected response kind")
            }
        }
    }
}

impl std::error::Error for McpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Connect(err) | Self::Io(err) => Some(err),
            Self::Wire(err) => Some(err),
            Self::Disconnected | Self::UnexpectedResponse | Self::Handshake(_) => None,
        }
    }
}
