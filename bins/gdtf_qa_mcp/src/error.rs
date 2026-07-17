//! [`McpError`] — the failure vocabulary for talking to the running game (GTW-741).

use core::fmt::{self, Display};
use std::io;

use gdtf_qa_protocol::framing::WireError;

/// A failure the MCP bridge meets while exchanging one request with the game's
/// `net_qa` server.
///
/// Distinct from a protocol-level [`QaError`](gdtf_qa_protocol::envelope::QaError) the
/// game deliberately returns (that rides back inside a normal
/// [`QaResponse`](gdtf_qa_protocol::envelope::QaResponse) and becomes an MCP tool error):
/// an [`McpError`] is the LINK itself failing — the game is not up, the socket broke, or
/// a frame would not encode/decode. The tool layer turns it into an MCP tool error so a
/// client sees the reason instead of a crash. The inner `io::Error` / [`WireError`] are
/// the underlying causes, not domain values.
#[derive(Debug)]
pub enum McpError {
    /// Could not open the loopback connection to the game (it is probably not running,
    /// or not built with the `net_qa` channel enabled).
    Connect(io::Error),
    /// A read or write on an established connection failed.
    Io(io::Error),
    /// A frame could not be encoded or decoded (the codec's typed error).
    Wire(WireError),
    /// The game closed the connection before answering the request.
    Disconnected,
    /// The game answered, but with a response variant that does not match the request
    /// that was sent (a protocol confusion).
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
            Self::Disconnected | Self::UnexpectedResponse => None,
        }
    }
}
