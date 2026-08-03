use bevy::prelude::{Deref, Entity, Message};

use crate::ganger::{Direction, StanceKind};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AimRequest(bool);

impl AimRequest {
        #[must_use]
    pub const fn new(aim: bool) -> Self {
        Self(aim)
    }
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetAimingRequested {
        pub actor: Entity,
        pub aim:   AimRequest,
}

impl SetAimingRequested {
        #[must_use]
    pub const fn new(actor: Entity, aim: AimRequest) -> Self {
        Self { actor, aim }
    }
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetStanceRequested {
        pub actor:  Entity,
        pub stance: StanceKind,
}

impl SetStanceRequested {
        #[must_use]
    pub const fn new(actor: Entity, stance: StanceKind) -> Self {
        Self { actor, stance }
    }
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetFacingRequested {
        pub actor:  Entity,
        pub facing: Direction,
}

impl SetFacingRequested {
        #[must_use]
    pub const fn new(actor: Entity, facing: Direction) -> Self {
        Self { actor, facing }
    }
}
