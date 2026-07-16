//! [`EventBatch`] + [`DroppedCount`] — the outbox drain wrapper (GTW-734).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use super::net_event::NetEvent;

/// How many events the game's bounded outbox **dropped** since the last drain — the
/// back-pressure signal.
///
/// The outbox is capacity-bounded (a slow client cannot make the game buffer without
/// limit); when it overflows it drops the oldest events and counts them, so a client
/// knows its view of the event stream has a gap. A private-inner newtype
/// (no-bare-types), serde-transparent over `u32`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DroppedCount(u32);

impl DroppedCount {
    /// Build a dropped-event count from its number.
    #[must_use]
    pub const fn new(dropped: u32) -> Self {
        Self(dropped)
    }
}

/// A drained batch of combat events — the reply to a
/// [`GetOutput`](crate::envelope::QaRequest::GetOutput).
///
/// The [`events`](Self::events) drained this request (up to the requested cap) plus the
/// [`dropped`](Self::dropped) count the bounded outbox shed since the last drain. Serde
/// default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventBatch {
    /// The events drained this request, in emission order.
    pub events:  Vec<NetEvent>,
    /// How many events the bounded outbox dropped since the last drain.
    pub dropped: DroppedCount,
}

impl EventBatch {
    /// Build an event batch from its drained events and the dropped count.
    #[must_use]
    pub const fn new(events: Vec<NetEvent>, dropped: DroppedCount) -> Self {
        Self { events, dropped }
    }
}
