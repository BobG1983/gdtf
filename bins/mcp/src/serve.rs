//! Hand this game's hosts and identity to the shared MCP server.

use crate::{hosts::registry, identity::identity};

/// Serve the registered hosts on stdin/stdout until EOF.
pub fn run() {
    cobalt_mcp_server::run_stdio(&identity(), registry());
}
