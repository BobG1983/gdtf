//! MCP system sets for the editor.

use bevy::prelude::*;

/// System sets for the editor MCP channel.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorMcpSystems {
    /// Gather, route, claim, and answer incoming requests.
    Gather,
}
