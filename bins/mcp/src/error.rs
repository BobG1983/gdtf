//! Errors from the MCP ↔ MCP bridge.

use core::fmt::{self, Display};
use std::io;

use cobalt_mcp_protocol::{framing::WireError, message::QaError};

/// Failure talking to a host over MCP.
#[derive(Debug)]
pub enum McpError {
    /// Could not open a TCP connection.
    Connect(io::Error),
    /// Read/write on an open connection failed.
    Io(io::Error),
    /// Frame encode/decode failed.
    Wire(WireError),
    /// Peer closed the connection.
    Disconnected,
    /// Hello handshake was refused.
    Handshake(QaError),
    /// Response kind did not match the request.
    UnexpectedResponse,
}

impl Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect(err) => write!(
                f,
                "could not connect to the game's mcp channel (is the game running \
                 with GDTF_MCP=1?): {err}"
            ),
            Self::Io(err) => write!(f, "mcp connection I/O failed: {err}"),
            Self::Wire(err) => write!(f, "mcp framing failed: {err}"),
            Self::Disconnected => f.write_str("the game closed the mcp connection"),
            Self::Handshake(err) => write!(
                f,
                "the mcp handshake was refused ({err:?}) — the child speaks a different \
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
