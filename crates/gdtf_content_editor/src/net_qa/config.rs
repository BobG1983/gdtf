//! This server's identity constants + its default listen port (GTW-804).
//!
//! The editor's own POLICY half of the activation gate: the protocol version it negotiates,
//! the name it identifies itself as, and the port it falls back to. The transport's own
//! vocabulary — [`NetQaPort`], [`NetIoTimeout`](gdtf_net_qa_transport::NetIoTimeout) and the
//! socket-timeout default — belongs to the shared `gdtf_net_qa_transport` crate, because both
//! hosts bind and time out the same way while choosing different ports.

use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::envelope::ProtocolVersion;

/// The wire protocol version this editor `net_qa` server speaks.
///
/// A [`Hello`](gdtf_qa_protocol::envelope::QaRequest::Hello) carrying THIS version negotiates
/// successfully ([`HelloOk`](gdtf_qa_protocol::envelope::QaResponse::HelloOk)); any other
/// version is rejected
/// [`VersionMismatch`](gdtf_qa_protocol::envelope::QaError::VersionMismatch). It tracks the
/// protocol crate's [`ProtocolVersion::CURRENT`] — the SAME contract the game negotiates, so
/// one QA client speaks to both hosts without a second version ladder to track.
pub(super) const EDITOR_QA_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::CURRENT;

/// The server's self-identifying name returned in the handshake facts.
///
/// Purely informational — a client logs (and can assert) which of the two hosts it reached,
/// which is exactly why it differs from the game's `gdtf-net-qa`. Not a domain value (a fixed
/// build label), so it stays a `&str` const here and is wrapped in a
/// [`ServerNameNet`](gdtf_qa_protocol::envelope::ServerNameNet) at the reply boundary.
///
/// `pub` (re-exported from the crate root under the same `net_qa` gate) so the integration
/// test asserts the negotiated identity without hard-coding the literal.
pub const EDITOR_QA_SERVER_NAME: &str = "gdtf-editor-net-qa";

/// The default loopback port the editor listener binds when `GDTF_EDITOR_NET_QA_PORT` is
/// unset or unparseable.
///
/// DELIBERATELY one above the game's `7616`: the editor and the game are separate binaries a
/// QA session may well run at the same time, and two hosts on one port means whichever starts
/// second fails to bind. Adjacent numbers keep the pair memorable while staying distinct.
pub(super) const DEFAULT_EDITOR_PORT: NetQaPort = NetQaPort::new(7617);
