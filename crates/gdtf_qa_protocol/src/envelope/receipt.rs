//! [`InjectReceipt`] + [`RejectReason`] — the reply to an injected intent (GTW-734).

use serde::{Deserialize, Serialize};

/// Why an injected [`NetIntent`](crate::intent::NetIntent) was **rejected** at route /
/// offer time.
///
/// The advisory route-time rejections the game side returns BEFORE the sim's own
/// dispatch gate (the sim gate stays the authoritative check; these are the wire-layer
/// pre-checks). An independent serde enum with at least the GTW-694-named variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RejectReason {
    /// No battle is in progress, so the intent has nothing to act on (rejected at route
    /// time via the `BattleInProgress` witness — the GTW-694 A-defect cure).
    NoBattle,
    /// A contextual act was pushed for a target the game is not currently OFFERING (the
    /// offer gate — the `ContextualOffer<A>` lacks the target).
    NotOffered,
    /// The target token does not resolve to a live entity (a stale / unknown handle).
    UnknownEntity,
    /// A [`Fire`](crate::intent::NetIntent::Fire) named a fire-mode index the weapon
    /// does not offer.
    BadFireMode,
    /// The token was minted for an entity that has since been despawned / replaced (a
    /// stale-generation handle).
    StaleToken,
    /// The intent drives an affordance this build does not have compiled in — currently
    /// only [`SwapUiStack`](crate::intent::NetIntent::SwapUiStack) against a build without
    /// the DEV UI-stack swap harness (GTW-816). Fail-closed and honest: the client is told
    /// the swap did not happen rather than being answered
    /// [`Queued`](InjectReceipt::Queued) for something that will never occur.
    Unavailable,
}

/// The reply to an [`Inject`](crate::envelope::QaRequest::Inject) — whether the intent
/// was queued or rejected.
///
/// [`Queued`](Self::Queued) means the intent was pushed onto the game's intent queue
/// (the sim's own dispatch gate is still the authoritative check that follows);
/// [`Rejected`](Self::Rejected) carries the wire-layer [`RejectReason`]. An independent
/// serde enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InjectReceipt {
    /// The intent was pushed onto the game's intent queue.
    Queued,
    /// The intent was rejected at the wire layer, with the reason.
    Rejected(RejectReason),
}
