//! Posture/orientation act requests — [`AimRequest`], [`SetAimingRequested`],
//! [`SetStanceRequested`], and [`SetFacingRequested`].

use bevy::prelude::{Deref, Entity, Message};

use crate::ganger::{Direction, StanceKind};

/// The requested **aim flag** carried by a [`SetAimingRequested`] — `true` for aimed
/// fire, `false` for hip-fired.
///
/// A no-bare-types newtype over the aim-mode `bool` payload (a domain value — the
/// requested aim mode, not framework plumbing): a private inner + a derived [`Deref`],
/// the crate's newtype house style. Distinct from the
/// [`Aiming`](crate::ganger::Aiming) *component* (the ganger's current aim state): this
/// is the *requested* value the dispatch verb sets the component to.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AimRequest(bool);

impl AimRequest {
    /// Build a requested aim flag — `true` to aim, `false` to hip-fire.
    #[must_use]
    pub const fn new(aim: bool) -> Self {
        Self(aim)
    }
}

/// A **set-aiming** act was requested — set `actor`'s aim flag to `aim`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor ref plus the requested
/// [`AimRequest`] flag. [`dispatch_set_aiming`](crate::acts::posture::dispatch_set_aiming)
/// calls [`set_aiming`](crate::posture::set_aiming) (which spends NO TU — toggling aim is
/// free, `posture.rs`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetAimingRequested {
    /// The acting ganger whose [`Aiming`](crate::ganger::Aiming) flag is set.
    pub actor: Entity,
    /// The requested aim value the flag is set to.
    pub aim:   AimRequest,
}

impl SetAimingRequested {
    /// Build a set-aiming request for `actor` to the requested aim flag.
    #[must_use]
    pub const fn new(actor: Entity, aim: AimRequest) -> Self {
        Self { actor, aim }
    }
}

/// A **set-stance** act was requested — change `actor`'s stance to `stance`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor ref plus the requested
/// [`StanceKind`]. [`dispatch_set_stance`](crate::acts::posture::dispatch_set_stance) calls
/// [`set_stance`](crate::posture::set_stance), which charges the
/// [`StanceChangeTu`](crate::tuning::StanceChangeTu) tuning leaf ONLY on a real change
/// (a no-op, no charge, when the actor already holds `stance`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetStanceRequested {
    /// The acting ganger whose [`Stance`](crate::ganger::Stance) is changed.
    pub actor:  Entity,
    /// The requested posture to change to.
    pub stance: StanceKind,
}

impl SetStanceRequested {
    /// Build a set-stance request for `actor` to change to `stance`.
    #[must_use]
    pub const fn new(actor: Entity, stance: StanceKind) -> Self {
        Self { actor, stance }
    }
}

/// A **set-facing** act was requested — turn `actor` to face `facing`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor ref plus the requested
/// [`Direction`]. [`dispatch_set_facing`](crate::acts::posture::dispatch_set_facing) calls
/// [`set_facing`](crate::posture::set_facing), which charges the
/// [`TurnTu`](crate::tuning::TurnTu) tuning leaf ONLY on a real turn (a no-op, no charge,
/// when the actor already faces `facing`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetFacingRequested {
    /// The acting ganger whose [`Facing`](crate::ganger::Facing) is turned.
    pub actor:  Entity,
    /// The requested direction to turn to.
    pub facing: Direction,
}

impl SetFacingRequested {
    /// Build a set-facing request for `actor` to turn to `facing`.
    #[must_use]
    pub const fn new(actor: Entity, facing: Direction) -> Self {
        Self { actor, facing }
    }
}
