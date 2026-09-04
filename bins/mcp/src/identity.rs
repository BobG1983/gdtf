//! The name and version this bridge advertises to a client.

use cobalt_mcp_server::{ServerIdentity, ServerName, ServerVersion};

// The name `.mcp.json` registers this server under.
const SERVER_NAME: &str = "gdtf-mcp";

/// What a client is told this server is when it connects.
#[must_use]
pub fn identity() -> ServerIdentity {
    ServerIdentity::new(
        ServerName::new(SERVER_NAME.to_owned()),
        ServerVersion::new(env!("CARGO_PKG_VERSION").to_owned()),
    )
}
