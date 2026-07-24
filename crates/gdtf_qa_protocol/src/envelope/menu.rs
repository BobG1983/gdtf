//! [`MenuActivationReceipt`] — the reply to an
//! [`ActivateMenuItem`](crate::envelope::QaRequest::ActivateMenuItem) (GTW-787).

use serde::{Deserialize, Serialize};

use super::receipt::RejectReason;

/// The reply to an [`ActivateMenuItem`](crate::envelope::QaRequest::ActivateMenuItem) —
/// whether the item's activation was dispatched or the token was rejected.
///
/// [`Activated`](Self::Activated) means the token resolved to a live, listed menu item and
/// its activation was raised through the game's real focus-activation path (the same
/// message an `Enter` keypress raises); the resulting state change is then observable via
/// [`GetAppFlow`](crate::envelope::QaRequest::GetAppFlow). [`Rejected`](Self::Rejected)
/// carries the reason — a token that names no live, listed menu item is
/// [`StaleToken`](RejectReason::StaleToken). Parallels
/// [`InjectReceipt`](crate::envelope::InjectReceipt); an independent serde enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MenuActivationReceipt {
    /// The item's activation was dispatched through the focus-activation path.
    Activated,
    /// The token was rejected at the wire layer, with the reason (a stale / unlisted
    /// token is [`StaleToken`](RejectReason::StaleToken)).
    Rejected(RejectReason),
}
