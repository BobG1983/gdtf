//! This server's identity constants + its default listen port (GTW-736; GTW-803 lifted the
//! transport's own typed config into `gdtf_net_qa_transport`).
//!
//! What stays here is THIS host's policy: the protocol version this server speaks, its
//! self-identifying name, and the port [`from_env`](super::plugin::NetQaPlugin::from_env)
//! falls back to. The transport's own vocabulary — [`NetQaPort`],
//! [`NetIoTimeout`](gdtf_net_qa_transport::NetIoTimeout) and the socket-timeout default —
//! belongs to the shared crate, because another host picks a different port from a
//! different switch but binds and times out the same way.

use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::envelope::{HelloFacts, ProtocolVersion, ServerNameNet};

crate::support_item! {
    /// The wire protocol version this `net_qa` server speaks.
    ///
    /// A [`Hello`](gdtf_qa_protocol::envelope::QaRequest::Hello) carrying THIS version
    /// negotiates successfully ([`HelloOk`](gdtf_qa_protocol::envelope::QaResponse::HelloOk));
    /// any other version is rejected
    /// [`VersionMismatch`](gdtf_qa_protocol::envelope::QaError::VersionMismatch). It tracks
    /// the protocol crate's [`ProtocolVersion::CURRENT`], which is bumped on any breaking
    /// envelope change (GTW-746 bumped it to `2` for the `AppFlowView.available` field;
    /// GTW-749 to `3` for the `ScreenshotAfter` request; GTW-727 to `4` for the
    /// `AppFlowView.caught_up` field + the `NotCaughtUp` error; GTW-763 to `5` for the
    /// `FogView` cell COUNTS; GTW-766 to `6` for the `StepperControl` request +
    /// `StepperInactive` error).
    /// Widened to `pub` under `test-support` so the routing test can assert the negotiated
    /// version without hard-coding a literal.
    const NET_QA_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::CURRENT;
}

crate::support_item! {
    /// The server's self-identifying name returned in the handshake facts.
    ///
    /// Purely informational — a client logs what it connected to. Not a domain value
    /// (a fixed build label), so it stays a bare `&str` const here and is wrapped in a
    /// [`ServerNameNet`] at the reply boundary.
    /// Widened to `pub` under `test-support` so the routing test can assert the identity this
    /// host hands the listener without hard-coding the literal.
    const SERVER_NAME: &str = "gdtf-net-qa";
}

/// The default loopback port the listener binds when `GDTF_NET_QA_PORT` is unset or
/// unparseable — an arbitrary high port unlikely to collide with a common service.
pub(super) const DEFAULT_PORT: NetQaPort = NetQaPort::new(7616);

crate::support_item! {
    /// The handshake facts THIS host answers a `Hello` with — the version it speaks paired
    /// with the name it identifies itself as.
    ///
    /// Handed to [`run_listener`](gdtf_net_qa_transport::run_listener) by
    /// [`NetQaPlugin`](super::plugin::NetQaPlugin), because the listener thread answers every
    /// `Hello` itself (GTW-940). That is why the transport takes the whole facts rather than a
    /// bare version: the version is shared with the editor, but [`SERVER_NAME`] is this host's
    /// alone. The router no longer negotiates anything — a `Hello` never reaches its inbox.
    ///
    /// Widened to `pub` under `test-support` so a test can assert the facts the game passes
    /// without restating the two literals.
    #[must_use]
    fn hello_facts() -> HelloFacts {
        HelloFacts::new(
            NET_QA_PROTOCOL_VERSION,
            ServerNameNet::new(SERVER_NAME.to_owned()),
        )
    }
}
