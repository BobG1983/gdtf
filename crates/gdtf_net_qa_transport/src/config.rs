use core::time::Duration;

use bevy::prelude::Deref;

pub const DEFAULT_IO_TIMEOUT: NetIoTimeout = NetIoTimeout::new(Duration::from_secs(5));

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetQaPort(u16);

impl NetQaPort {
        #[must_use]
    pub const fn new(port: u16) -> Self {
        Self(port)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetIoTimeout(Duration);

impl NetIoTimeout {
        #[must_use]
    pub const fn new(timeout: Duration) -> Self {
        Self(timeout)
    }
}
