//! The typed transport configuration + server-identity constants for the QA net
//! layer (GTW-736).
//!
//! Small no-bare-types wrappers ([`NetQaPort`] over the TCP port, [`NetIoTimeout`]
//! over the two-sided socket timeout) plus the constants the router and listener
//! read: the protocol version this server speaks, its self-identifying name, and the
//! defaults [`from_env`](super::plugin::NetQaPlugin::from_env) falls back to.

use core::time::Duration;

use bevy::prelude::Deref;
use gdtf_qa_protocol::envelope::ProtocolVersion;

crate::support_item! {
    /// The wire protocol version this `net_qa` server speaks.
    ///
    /// A [`Hello`](gdtf_qa_protocol::envelope::QaRequest::Hello) carrying THIS version
    /// negotiates successfully ([`HelloOk`](gdtf_qa_protocol::envelope::QaResponse::HelloOk));
    /// any other version is rejected
    /// [`VersionMismatch`](gdtf_qa_protocol::envelope::QaError::VersionMismatch). Bumped
    /// on any breaking envelope change. Widened to `pub` under `test-support` so the
    /// routing test can assert the negotiated version without hard-coding a literal.
    const NET_QA_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::new(1);
}

/// The server's self-identifying name returned in the handshake facts.
///
/// Purely informational — a client logs what it connected to. Not a domain value
/// (a fixed build label), so it stays a bare `&str` const here and is wrapped in a
/// [`ServerNameNet`](gdtf_qa_protocol::envelope::ServerNameNet) at the reply boundary.
pub(super) const SERVER_NAME: &str = "gdtf-net-qa";

/// The default loopback port the listener binds when `GDTF_NET_QA_PORT` is unset or
/// unparseable — an arbitrary high port unlikely to collide with a common service.
pub(super) const DEFAULT_PORT: NetQaPort = NetQaPort::new(7616);

/// The default two-sided socket timeout — the read/write deadline set on the client
/// socket so a stuck or idle client cannot hold the single client slot forever.
pub(super) const DEFAULT_IO_TIMEOUT: NetIoTimeout = NetIoTimeout::new(Duration::from_secs(5));

crate::support_item! {
    /// A loopback TCP **port** the `net_qa` listener binds on.
    ///
    /// Private-inner newtype over `u16` (no-bare-types). The interface is ALWAYS
    /// [`Ipv4Addr::LOCALHOST`](std::net::Ipv4Addr::LOCALHOST) — only the port is
    /// configurable (via `GDTF_NET_QA_PORT`). A bind on port `0` asks the OS for a free
    /// ephemeral port, which the transport reads back — the deterministic-test recipe.
    /// Widened to `pub` under `test-support` (the transport test derefs it),
    /// `pub(crate)` otherwise.
    #[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct NetQaPort(u16);
}

impl NetQaPort {
    /// Build a port from its number.
    #[must_use]
    pub(super) const fn new(port: u16) -> Self {
        Self(port)
    }
}

crate::support_item! {
    /// The two-sided (read AND write) **timeout** set on the accepted client socket.
    ///
    /// Private-inner newtype over [`Duration`] (no-bare-types). A read that blocks this
    /// long reaps the client (freeing the single slot); a write that blocks this long
    /// treats the client as gone. The listener sets it on both directions of the socket.
    /// Widened to `pub` under `test-support` (the transport test pins a short timeout),
    /// `pub(crate)` otherwise.
    #[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct NetIoTimeout(Duration);
}

impl NetIoTimeout {
    crate::support_item! {
        /// Build a socket timeout from its duration.
        ///
        /// Widened to `pub` under `test-support` so the transport test can pin a short
        /// timeout and observe the reap deterministically.
        #[must_use]
        const fn new(timeout: Duration) -> Self {
            Self(timeout)
        }
    }
}
