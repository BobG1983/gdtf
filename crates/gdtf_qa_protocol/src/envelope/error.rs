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
    /// The request needs the DEV-ONLY procgen stepper to be actively driving (a live
    /// `StagedProcgen`), and it is not — no drive is in flight (GTW-766).
    ///
    /// The route-time refusal of a
    /// [`StepperControl`](crate::envelope::QaRequest::StepperControl): the stepper is
    /// drivable only during a battle's procgen `Generation` while it is engaged, and outside
    /// that window this is the accurate reason — never a
    /// [`NoBattle`](Self::NoBattle) lie (a battle may well be running) nor
    /// [`NotCaughtUp`](Self::NotCaughtUp). A client polls
    /// [`AppFlowView::available`](crate::view::AppFlowView) to learn when the request is
    /// serviceable.
    StepperInactive,
    /// The frame did not decode as a [`QaRequest`](crate::envelope::QaRequest) (GTW-939).
    ///
    /// A DECODE failure, distinct from [`BadRequest`](Self::BadRequest): the bytes never
    /// became a request at all, so no host state was consulted and none could be. Nothing
    /// writes this value yet: today `handle_frame` answers a decode failure with
    /// [`BadRequest`](Self::BadRequest). GTW-940 is the ticket that makes the transport
    /// answer `Malformed` in the listener thread, before the host's inbox is reached; the
    /// variant lands here first so that ticket has a value to write.
    Malformed,
    /// A request other than [`Hello`](crate::envelope::QaRequest::Hello) arrived before the
    /// protocol version was negotiated (GTW-939).
    ///
    /// The enforcement the handshake lacked: the version was carried, replied to, and then
    /// never consulted again, so a client speaking a stale envelope got a `HelloOk` and then
    /// silently mis-decoded every later reply. Nothing writes this value yet: GTW-940 is the
    /// ticket that makes the transport answer it for every non-`Hello` frame on a connection
    /// that has not negotiated. The variant lands here first so that ticket has a value to
    /// write.
    NotNegotiated,
}
