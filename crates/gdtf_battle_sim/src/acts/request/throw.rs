//! Throw grenade request and resolution message.

use bevy::prelude::{Entity, Message};

use crate::{metric::CellLevel, weapon::DamageType};

/// Request to throw a grenade at a cell.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThrowGrenadeRequested {
    /// Thrower.
    pub thrower: Entity,
    /// Target cell/level.
    pub target: CellLevel,
}

impl ThrowGrenadeRequested {
    /// Build a throw request.
    #[must_use]
    pub const fn new(thrower: Entity, target: CellLevel) -> Self {
        Self { thrower, target }
    }
}

/// Grenade landed / resolved at a cell.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThrowResolved {
    /// Impact cell.
    pub at: CellLevel,
    /// Damage type of the blast.
    pub damage: DamageType,
}

impl ThrowResolved {
    /// Build a throw-resolved message.
    #[must_use]
    pub const fn new(at: CellLevel, damage: DamageType) -> Self {
        Self { at, damage }
    }
}
