//! The TCP port a MCP listener or client speaks on. Each host chooses its own number.

use bevy_derive::Deref;

/// Command-line flag a launcher hands a host to name the port its listener binds.
///
/// The value follows as the next argument. A host started without it opens no listener.
pub const MCP_PORT_FLAG: &str = "--mcp-port";

/// TCP port used by a MCP listener or client.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct McpPort(u16);

impl McpPort {
    /// Wrap a port number.
    #[must_use]
    pub const fn new(port: u16) -> Self {
        Self(port)
    }
}
