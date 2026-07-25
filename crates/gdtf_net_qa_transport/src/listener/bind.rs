//! The loopback bind: interface policy plus the ephemeral-port readback (GTW-736).

use std::{
    io,
    net::{Ipv4Addr, TcpListener},
};

use crate::config::NetQaPort;

/// Bind the loopback listener on `port`, returning it and the ACTUAL port bound.
///
/// The interface is ALWAYS [`Ipv4Addr::LOCALHOST`]. A `port` of `0` asks the OS for a
/// free ephemeral port, which [`local_addr`](TcpListener::local_addr) reads back — the
/// deterministic-test recipe (no fixed port to collide on).
///
/// # Errors
///
/// Any [`io::Error`] from [`TcpListener::bind`] / [`TcpListener::local_addr`] (e.g. the
/// requested port already in use).
pub fn bind_listener(port: NetQaPort) -> io::Result<(TcpListener, NetQaPort)> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, *port))?;
    let actual = NetQaPort::new(listener.local_addr()?.port());
    Ok((listener, actual))
}
