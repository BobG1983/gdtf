//! Melee attack request and outcome messages.

use bevy::prelude::{Entity, Message};

use crate::{metric::CellLevel, resolve_hit::HpDamage, weapon::DamageType};

/// Melee target: ganger or structure cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeleeTarget {
    /// Living combatant.
    Ganger(Entity),
    /// Terrain structure at a cell.
    Structure(CellLevel),
}

/// Request a melee attack.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeleeRequested {
    /// Attacker.
    pub attacker: Entity,
    /// Target.
    pub target:   MeleeTarget,
}

impl MeleeRequested {
    /// Attack a ganger.
    #[must_use]
    pub const fn new(attacker: Entity, target: Entity) -> Self {
        Self {
            attacker,
            target: MeleeTarget::Ganger(target),
        }
    }

    /// Attack a structure cell.
    #[must_use]
    pub const fn new_structural(attacker: Entity, at: CellLevel) -> Self {
        Self {
            attacker,
            target: MeleeTarget::Structure(at),
        }
    }
}

/// Melee resolved against a structure (for FX / logging).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeleeResolved {
    /// Cell struck.
    pub at:     CellLevel,
    /// Damage type used.
    pub damage: DamageType,
}

impl MeleeResolved {
    /// Build a resolved-structure message.
    #[must_use]
    pub const fn new(at: CellLevel, damage: DamageType) -> Self {
        Self { at, damage }
    }
}

/// Melee struck a ganger for HP damage.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeleeStruck {
    /// Attacker.
    pub attacker:  Entity,
    /// Target.
    pub target:    Entity,
    /// HP damage dealt.
    pub hp_damage: HpDamage,
}

impl MeleeStruck {
    /// Build a struck message.
    #[must_use]
    pub const fn new(attacker: Entity, target: Entity, hp_damage: HpDamage) -> Self {
        Self {
            attacker,
            target,
            hp_damage,
        }
    }
}
