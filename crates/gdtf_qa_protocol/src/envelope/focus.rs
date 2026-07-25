//! [`FocusStepNet`] / [`FocusCommandNet`] — the focus-drive command, and
//! [`FocusControlReceipt`] — the reply to a focus-control request (GTW-802).

use serde::{Deserialize, Serialize};

use super::receipt::RejectReason;
use crate::ids::FocusTargetNet;

/// One step of focus movement — the wire mirror of an arrow-key / D-pad press.
///
/// [`Next`](Self::Next) / [`Prev`](Self::Prev) walk the screen's vertical focus chain (the
/// direction the down / up arrow moves); [`Left`](Self::Left) / [`Right`](Self::Right) walk
/// the horizontal one. An independent, closed serde enum — never a leak of the game's own
/// direction type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FocusStepNet {
    /// Move focus to the next control down the chain (the down arrow / D-pad down).
    Next,
    /// Move focus to the previous control up the chain (the up arrow / D-pad up).
    Prev,
    /// Move focus one control to the left (the left arrow).
    Left,
    /// Move focus one control to the right (the right arrow).
    Right,
}

/// A single focus-drive command a QA client sends — move focus, point focus, or activate
/// (GTW-802).
///
/// Carried by [`QaRequest::FocusControl`](crate::envelope::QaRequest::FocusControl). Every
/// variant is realised through the game's REAL focus/interaction path: a
/// [`Step`](Self::Step) writes the SAME navigate message an arrow key writes and lets the
/// game's own navigation system move the focus, and an activation emits a real `Enter`
/// keypress at the focused control, so the game's own bridges raise their own activation
/// signals. The router never forges an activation message and never writes a state
/// transition itself. An independent, closed serde enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FocusCommandNet {
    /// Move focus one step in a direction, exactly as an arrow key would.
    Step(FocusStepNet),
    /// Point focus at one enumerated control by its token, without activating it.
    Focus(FocusTargetNet),
    /// Activate whatever control currently holds focus.
    Activate,
    /// Point focus at one enumerated control by its token, then activate it — the
    /// "click this control" command.
    ActivateTarget(FocusTargetNet),
}

/// The reply to a [`FocusControl`](crate::envelope::QaRequest::FocusControl) — whether the
/// command was dispatched or the token was rejected (GTW-802).
///
/// [`Applied`](Self::Applied) means the command was dispatched down the game's real focus /
/// input path. Like [`InjectReceipt::Queued`](crate::envelope::InjectReceipt::Queued) it
/// says nothing about the eventual outcome: an activation is realised as a real keypress
/// the input stack consumes on the FOLLOWING frame, so the resulting state change is
/// observed with a follow-up [`GetAppFlow`](crate::envelope::QaRequest::GetAppFlow).
/// [`Rejected`](Self::Rejected) carries the reason — a token that names no currently listed
/// focusable (and an [`Activate`](FocusCommandNet::Activate) with nothing focused) is
/// [`StaleToken`](RejectReason::StaleToken). An independent serde enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FocusControlReceipt {
    /// The command was dispatched through the game's real focus / input path.
    Applied,
    /// The command was rejected at the wire layer, with the reason (a stale / unlisted
    /// token is [`StaleToken`](RejectReason::StaleToken)).
    Rejected(RejectReason),
}
