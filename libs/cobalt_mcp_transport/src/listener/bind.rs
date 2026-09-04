//! Bind a localhost TCP listener for MCP.

use std::{
    io,
    net::{Ipv4Addr, TcpListener},
};

use cobalt_mcp_protocol::ports::McpPort;

/// Bind on localhost at `port` (or an ephemeral port if `0`).
///
/// # Errors
///
/// Returns I/O errors from bind or reading the local address.
pub fn bind_listener(port: McpPort) -> io::Result<(TcpListener, McpPort)> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, *port))?;
    let actual = McpPort::new(listener.local_addr()?.port());
    Ok((listener, actual))
}
