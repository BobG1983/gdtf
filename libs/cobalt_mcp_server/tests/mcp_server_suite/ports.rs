//! Loopback ports the suite hands out, in two ranges that cannot collide.
//!
//! A fixture listener takes an OS ephemeral port; a free port comes from below every floor.

use std::{
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    sync::atomic::{AtomicU16, Ordering},
};

// Below every host's ephemeral floor: 32768 on Linux, 49152 on macOS and Windows.
static NEXT_FREE: AtomicU16 = AtomicU16::new(20_000);

/// The port a listener is bound to.
pub(crate) fn port_of(listener: &TcpListener) -> u16 {
    let Ok(addr) = listener.local_addr() else {
        unreachable!("the listener has a local address");
    };
    addr.port()
}

/// A loopback listener on an OS-chosen ephemeral port.
pub(crate) fn bind_loopback() -> TcpListener {
    let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) else {
        unreachable!("the test can bind a loopback listener");
    };
    listener
}

/// A loopback port nothing in the suite listens on, and no two cases share.
///
/// Checked by connecting, never by binding: a forked child would inherit a bound socket.
pub(crate) fn issue_free_port() -> u16 {
    loop {
        let port = NEXT_FREE.fetch_add(1, Ordering::SeqCst);
        if port == 0 {
            unreachable!("the suite issued more free ports than the range holds");
        }
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        if TcpStream::connect(addr).is_err() {
            return port;
        }
    }
}
