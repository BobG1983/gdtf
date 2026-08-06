//! Bind a localhost TCP listener for net QA.

use std::{
    io,
    net::{Ipv4Addr, TcpListener},
};

use gdtf_qa_protocol::ports::NetQaPort;

/// Bind on localhost at `port` (or an ephemeral port if `0`).
///
/// # Errors
///
/// Returns I/O errors from bind or reading the local address.
pub fn bind_listener(port: NetQaPort) -> io::Result<(TcpListener, NetQaPort)> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, *port))?;
    let actual = NetQaPort::new(listener.local_addr()?.port());
    Ok((listener, actual))
}
