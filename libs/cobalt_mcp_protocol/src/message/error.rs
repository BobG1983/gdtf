//! High-level MCP session errors (not wire framing).

use serde::{Deserialize, Serialize};

/// Protocol-level failure reported in a response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum McpSessionError {
    /// Request body could not be understood.
    Malformed,
    /// Client has not completed hello negotiation.
    NotNegotiated,
    /// Client and server protocol versions differ.
    VersionMismatch,
    /// Server cannot accept the request right now.
    Busy,
    /// Operation timed out.
    Timeout,
}
