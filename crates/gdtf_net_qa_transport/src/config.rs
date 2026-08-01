//! The typed transport configuration for the QA net layer (GTW-736; lifted in GTW-803).
//!
//! Small no-bare-types wrappers — [`NetQaPort`] over the TCP port the listener binds,
//! [`NetIoTimeout`] over the two-sided socket timeout it sets — plus the timeout default.
//! Both appear in the listener's own signatures, so they are the transport's configuration
//! vocabulary rather than any one host's activation policy: which port a host chooses, and
//! from which environment variable, stays with that host.

use core::time::Duration;

use bevy::prelude::Deref;

/// The default two-sided socket timeout — the read/write deadline set on the client
/// socket so a stuck or idle client cannot hold the single client slot forever.
///
/// The ONE place this number lives (the QA protocol rewrite's `QA_IO_TIMEOUT`). It is a
/// DEFAULT, not a fixed value: [`run_listener`](crate::run_listener) takes a
/// [`NetIoTimeout`] parameter, so a host — or a test — passes whatever it needs and only the
/// fallback is edited here.
///
/// **Open question Q3 is UNRESOLVED.** The rewrite's design proposes raising this to 180 s
/// (with the courier's `LINK_TIMEOUT` at 200 s) so the command layer's 120 s
/// `MAX_AWAIT_BUDGET` sits strictly inside both and a long wait answers its own deadline
/// error instead of surfacing as a socket
/// [`Timeout`](gdtf_qa_protocol::message::QaError::Timeout); the counter-lever is halving
/// that budget to 60 s and taking the two timeouts to 90 s / 100 s. No user ruling has been
/// made, so the value stays at the 5 s it has always been until one is.
pub const DEFAULT_IO_TIMEOUT: NetIoTimeout = NetIoTimeout::new(Duration::from_secs(5));

/// A loopback TCP **port** the QA listener binds on.
///
/// Private-inner newtype over `u16` (no-bare-types). The interface is ALWAYS
/// [`Ipv4Addr::LOCALHOST`](std::net::Ipv4Addr::LOCALHOST) — only the port is
/// configurable. A bind on port `0` asks the OS for a free ephemeral port, which the
/// transport reads back — the deterministic-test recipe.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetQaPort(u16);

impl NetQaPort {
    /// Build a port from its number.
    #[must_use]
    pub const fn new(port: u16) -> Self {
        Self(port)
    }
}

/// The two-sided (read AND write) **timeout** set on the accepted client socket.
///
/// Private-inner newtype over [`Duration`] (no-bare-types). A read that blocks this
/// long reaps the client (freeing the single slot); a write that blocks this long
/// treats the client as gone. The listener sets it on both directions of the socket.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetIoTimeout(Duration);

impl NetIoTimeout {
    /// Build a socket timeout from its duration.
    #[must_use]
    pub const fn new(timeout: Duration) -> Self {
        Self(timeout)
    }
}
