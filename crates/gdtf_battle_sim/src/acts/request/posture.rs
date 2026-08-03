//! Requests to change aiming, stance, or facing.

use bevy::prelude::{Deref, Entity, Message};

use crate::ganger::{Direction, StanceKind};

/// Desired aiming state.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AimRequest(bool);

impl AimRequest {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(aim: bool) -> Self {
        Self(aim)
    }
}

/// Set aiming on or off.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetAimingRequested {
    /// Actor.
    pub actor: Entity,
    /// Aim on/off.
    pub aim: AimRequest,
}

impl SetAimingRequested {
    /// Build a set-aiming request.
    #[must_use]
    pub const fn new(actor: Entity, aim: AimRequest) -> Self {
        Self { actor, aim }
    }
}

/// Change stance.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetStanceRequested {
    /// Actor.
    pub actor: Entity,
    /// Target stance.
    pub stance: StanceKind,
}

impl SetStanceRequested {
    /// Build a set-stance request.
    #[must_use]
    pub const fn new(actor: Entity, stance: StanceKind) -> Self {
        Self { actor, stance }
    }
}

/// Change facing.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetFacingRequested {
    /// Actor.
    pub actor: Entity,
    /// Target direction.
    pub facing: Direction,
}

impl SetFacingRequested {
    /// Build a set-facing request.
    #[must_use]
    pub const fn new(actor: Entity, facing: Direction) -> Self {
        Self { actor, facing }
    }
}
