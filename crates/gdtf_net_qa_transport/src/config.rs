//! Port and I/O timeout types for the net QA transport.

use core::time::Duration;

use bevy::prelude::Deref;

/// Default read/write timeout for net QA sockets.
pub const DEFAULT_IO_TIMEOUT: NetIoTimeout = NetIoTimeout::new(Duration::from_secs(5));

/// TCP port used by a net QA listener or client.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetQaPort(u16);

impl NetQaPort {
    /// Wrap a port number.
    #[must_use]
    pub const fn new(port: u16) -> Self {
        Self(port)
    }
}

/// Socket read/write timeout.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetIoTimeout(Duration);

impl NetIoTimeout {
    /// Wrap a duration.
    #[must_use]
    pub const fn new(timeout: Duration) -> Self {
        Self(timeout)
    }
}
