//! [`QaError`] — the protocol-level error vocabulary (GTW-734).

use serde::{Deserialize, Serialize};

/// A protocol-level **error** returned instead of a normal reply.
///
/// The wire failures a request can meet before (or instead of) producing its payload.
/// Distinct from an [`InjectReceipt::Rejected`](crate::envelope::InjectReceipt::Rejected)
/// (which is a NORMAL reply to an inject that the sim gate turned down) — a [`QaError`]
/// is the request itself failing. An independent serde enum with at least the
/// GTW-694-named variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QaError {
    /// The server is busy servicing another request and cannot take this one now.
    Busy,
    /// The client's [`Hello`](crate::envelope::QaRequest::Hello) protocol version does
    /// not match the server's.
    VersionMismatch,
    /// The request needs a live battle but none is in progress.
    NoBattle,
    /// The request was malformed or nonsensical in the current state.
    BadRequest,
}
