//! The TCP port a MCP listener or client speaks on. Each host chooses its own number.

use bevy_derive::Deref;

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
