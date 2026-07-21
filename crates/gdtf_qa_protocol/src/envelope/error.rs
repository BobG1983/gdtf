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
    /// The request needs the game to have finished SHOWING what already happened, and it
    /// has not (GTW-727 C43).
    ///
    /// The presenter is mid-playback: an exchange is being replayed on screen and the game
    /// is refusing act-bearing input until it finishes, exactly as it refuses the player's.
    /// Distinct from [`NoBattle`](Self::NoBattle), which would be a lie here — the battle is
    /// very much running. A client either waits for
    /// [`AppFlowView::caught_up`](crate::view::AppFlowView) or simply polls and retries.
    NotCaughtUp,
    /// The request was accepted but no consumer answered it before its
    /// deadline — the game side swept it and returned this instead of leaving
    /// the client hanging (the GTW-694 `FrameDeadline` sweep; GTW-736).
    Timeout,
}
