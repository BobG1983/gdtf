//! The read-derived **available hand count** [`HandsAvailable`] — the projection of
//! the injury ledger the shared `can_fire` guard checks (GTW-443; split out of
//! `ledger.rs` in GTW-550's palette re-home).

use bevy::prelude::Deref;

/// A ganger's **available hand count** — how many working hands it currently has
/// (`0..=2`), the read-derived input the shared `can_fire` guard checks a
/// [`TwoHanded`](crate::weapon::Handedness::TwoHanded) weapon against (GTW-443).
///
/// DERIVED-ON-READ from the [`InflictedInjuries`](super::InflictedInjuries) ledger via
/// [`hands_available`](super::InflictedInjuries::hands_available) — NOT a stored component and
/// NOT a [`StatTarget`](super::StatTarget) slot: a hand-disabling injury surfaces here by folding
/// the ledger's [`gained`](super::InflictedInjuries::gained) entries (the single-source-of-truth,
/// so a content hot-edit re-derives it rather than wiping an applied-once counter).
/// Default = `HandsAvailable(2)` (no injuries, both hands working).
///
/// A no-bare-types newtype (a hand count is a domain value): private inner + derived
/// [`Deref`]; the constructor [`new`](HandsAvailable::new) clamps into `0..=2`, so an
/// out-of-range count can never exist. Derives [`Hash`] / [`Eq`] / [`Copy`] so it can be
/// a value field of the [`FireActor`](crate::magazine::FireActor) read-bundle.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandsAvailable(u8);

impl HandsAvailable {
    /// The maximum hand count — a ganger has two hands. Reused by the `can_fire`
    /// hand-count clause as the [`TwoHanded`](crate::weapon::Handedness::TwoHanded) need.
    pub(crate) const MAX: u8 = 2;

    /// Build a hand count, **clamping** into `0..=2` (a ganger can never have more than
    /// two working hands, nor a negative count).
    #[must_use]
    pub const fn new(hands: u8) -> Self {
        Self(if hands > Self::MAX { Self::MAX } else { hands })
    }
}

impl Default for HandsAvailable {
    /// Two working hands — the uninjured default (no ledger / no arm injury).
    fn default() -> Self {
        Self(Self::MAX)
    }
}
