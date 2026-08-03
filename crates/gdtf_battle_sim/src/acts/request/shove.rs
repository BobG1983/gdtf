//! Deliberate or weapon-triggered shove.

use bevy::prelude::{Entity, Message};

/// Shove request.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShoveRequested {
    /// Who is shoving.
    pub shover: Entity,
    /// Who is shoved.
    pub target: Entity,
    /// How the shove was initiated.
    pub source: ShoveSource,
}

/// Origin of a shove.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShoveSource {
    /// Player/AI deliberate shove act.
    Deliberate,
    /// Secondary effect of a weapon hit.
    Weapon,
}

impl ShoveRequested {
    /// Deliberate shove.
    #[must_use]
    pub const fn new(shover: Entity, target: Entity) -> Self {
        Self {
            shover,
            target,
            source: ShoveSource::Deliberate,
        }
    }

    /// Weapon-triggered shove.
    #[must_use]
    pub const fn new_weapon(shover: Entity, target: Entity) -> Self {
        Self {
            shover,
            target,
            source: ShoveSource::Weapon,
        }
    }
}
