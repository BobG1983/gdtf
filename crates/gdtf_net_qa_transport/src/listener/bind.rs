use std::{
    io,
    net::{Ipv4Addr, TcpListener},
};

use crate::config::NetQaPort;

pub fn bind_listener(port: NetQaPort) -> io::Result<(TcpListener, NetQaPort)> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, *port))?;
    let actual = NetQaPort::new(listener.local_addr()?.port());
    Ok((listener, actual))
}
