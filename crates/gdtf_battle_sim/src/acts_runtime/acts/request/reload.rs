//! The reload act request — [`ReloadRequested`].

use bevy::prelude::{Entity, Message};

/// A **reload** act was requested — refill `actor`'s magazine, charging the weapon's
/// per-weapon reload TU cost (GTW-275).
///
/// A buffered [`Message`] carrying ONLY the [`Entity`] actor ref — the cost (the
/// [`ReloadTu`](crate::magazine::ReloadTu)) and the target fill (the
/// [`MagazineSize`](crate::weapon::MagazineSize)) both live on the actor's own
/// [`Magazine`](crate::magazine::Magazine) grouping component, so the message needs no
/// payload (the [`SetStanceRequested`](super::posture::SetStanceRequested) shape, minus the requested value).
/// [`dispatch_reload`](crate::acts::reload::dispatch_reload) fetches the actor's
/// `(&mut Magazine, &mut Tu, &LifeState)`, gates on alive + affordable, then spends the
/// magazine's own `reload_tu` and refills it to full. The actor is a Bevy [`Entity`]
/// handle — framework plumbing, the only bare type the no-bare-types rule permits in a
/// payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReloadRequested {
    /// The acting ganger whose [`Magazine`](crate::magazine::Magazine) is reloaded.
    pub actor: Entity,
}

impl ReloadRequested {
    /// Build a reload request for `actor`.
    #[must_use]
    pub const fn new(actor: Entity) -> Self {
        Self { actor }
    }
}
