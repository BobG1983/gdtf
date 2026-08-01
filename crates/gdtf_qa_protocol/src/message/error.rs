//! [`QaError`] — the protocol-level error vocabulary (GTW-734, cut to five by GTW-943).

use serde::{Deserialize, Serialize};

/// A protocol-level **error** returned instead of a normal reply.
///
/// The wire failures a request can meet before (or instead of) producing its payload.
/// Distinct from a command saying no: a command that runs and refuses does so inside its
/// own declared reply type, and a command that cannot run right now is answered
/// [`CommandOutcome::Unavailable`](crate::command::CommandOutcome::Unavailable). A
/// [`QaError`] is the REQUEST failing, before any command is reached.
///
/// Five variants, and the set is closed — `message/test/freeze.rs` pins it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QaError {
    /// The frame did not decode as a [`QaRequest`](crate::message::QaRequest).
    ///
    /// A DECODE failure: the bytes never became a request at all, so no host state was
    /// consulted and none could be. The transport writes it in the listener thread, before
    /// the host's inbox is reached (GTW-940).
    Malformed,
    /// A request other than [`Hello`](crate::message::QaRequest::Hello) arrived before the
    /// protocol version was negotiated (GTW-939).
    ///
    /// The enforcement the handshake lacked: the version used to be carried, replied to,
    /// and then never consulted again, so a client speaking a stale wire got a `HelloOk`
    /// and then silently mis-decoded every later reply. The transport answers it for every
    /// non-`Hello` frame on a connection that has not negotiated (GTW-940).
    NotNegotiated,
    /// The client's [`Hello`](crate::message::QaRequest::Hello) protocol version does not
    /// match the server's.
    VersionMismatch,
    /// The server is already serving another client and cannot take this one now.
    Busy,
    /// The request was accepted but no consumer answered it before its deadline — the host
    /// swept it and returned this instead of leaving the client hanging (the GTW-694
    /// `FrameDeadline` sweep; GTW-736).
    Timeout,
}
