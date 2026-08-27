//! Timeouts both ends of the net QA wire agree on.

use core::time::Duration;

use bevy_derive::Deref;

/// Default read/write timeout for net QA sockets.
pub const DEFAULT_IO_TIMEOUT: NetIoTimeout = NetIoTimeout::new(Duration::from_secs(5));

/// Default limit on how long a host may take to answer a forwarded request.
pub const DEFAULT_REPLY_TIMEOUT: NetReplyTimeout = NetReplyTimeout::new(Duration::from_secs(180));

/// Socket read/write timeout, which bounds how long an idle client holds the channel.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetIoTimeout(Duration);

impl NetIoTimeout {
    /// Wrap a duration.
    #[must_use]
    pub const fn new(timeout: Duration) -> Self {
        Self(timeout)
    }
}

/// How long the listener waits for the host to answer a forwarded request.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetReplyTimeout(Duration);

impl NetReplyTimeout {
    /// Wrap a duration.
    #[must_use]
    pub const fn new(timeout: Duration) -> Self {
        Self(timeout)
    }
}

/// The two timeouts a listener runs with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetTimeouts {
    io:    NetIoTimeout,
    reply: NetReplyTimeout,
}

impl NetTimeouts {
    /// The shipped pair: a short socket timeout and a long reply wait.
    pub const DEFAULT: Self = Self::new(DEFAULT_IO_TIMEOUT, DEFAULT_REPLY_TIMEOUT);

    /// The pair an in-process test listener runs with, whose read wait never fires.
    /// Such a client runs app frames between reads, so the shipped reap would cut it off.
    pub const NO_IDLE_REAP: Self =
        Self::new(NetIoTimeout::new(Duration::MAX), DEFAULT_REPLY_TIMEOUT);

    /// Pair a socket timeout with a reply wait.
    #[must_use]
    pub const fn new(io: NetIoTimeout, reply: NetReplyTimeout) -> Self {
        Self { io, reply }
    }

    /// The socket read/write timeout.
    #[must_use]
    pub const fn io(&self) -> NetIoTimeout {
        self.io
    }

    /// The wait for the host's reply.
    #[must_use]
    pub const fn reply(&self) -> NetReplyTimeout {
        self.reply
    }
}
