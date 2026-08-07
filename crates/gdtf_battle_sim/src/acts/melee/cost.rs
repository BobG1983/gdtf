//! TU cost and reach queries for a melee strike.

use bevy::prelude::Deref;

use crate::{
    acts::downed::is_8_adjacent,
    ganger::{Faction, LifeState, Position, Tu},
    metric::CellLevel,
    weapon::FightMode,
};

/// Whether the attacker may strike the requested target.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanMelee(bool);

impl CanMelee {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

/// Attacker geometry and allegiance for the reach check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeleeAttacker {
    /// Position.
    pub position: Position,
    /// Faction.
    pub faction:  Faction,
}

impl MeleeAttacker {
    /// Build an attacker snapshot from position and faction.
    #[must_use]
    pub const fn new(position: Position, faction: Faction) -> Self {
        Self { position, faction }
    }
}

/// What the strike is aimed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeleeReach {
    /// Another combatant.
    Ganger {
        /// Position.
        position: Position,
        /// Faction.
        faction:  Faction,
        /// Life state.
        life:     LifeState,
    },
    /// A structure or cover cell.
    Structure {
        /// Cell being smashed.
        at: CellLevel,
    },
}

impl MeleeReach {
    /// Aim the strike at a combatant.
    #[must_use]
    pub const fn ganger(position: Position, faction: Faction, life: LifeState) -> Self {
        Self::Ganger {
            position,
            faction,
            life,
        }
    }

    /// Aim the strike at a structure cell.
    #[must_use]
    pub const fn structure(at: CellLevel) -> Self {
        Self::Structure { at }
    }
}

/// TU charged for one strike with this weapon's primary mode.
#[must_use]
pub fn melee_tu_cost(fight_mode: &FightMode) -> Tu {
    Tu::new(u8::try_from(*fight_mode.primary().tu_cost).unwrap_or(u8::MAX))
}

/// Adjacent, and for a ganger target also hostile and still active.
#[must_use]
pub fn can_melee(attacker: MeleeAttacker, target: MeleeReach) -> CanMelee {
    match target {
        MeleeReach::Ganger {
            position,
            faction,
            life,
        } => CanMelee::new(
            *is_8_adjacent(attacker.position, position)
                && attacker.faction != faction
                && *life.is_active(),
        ),
        MeleeReach::Structure { at } => {
            CanMelee::new(*is_8_adjacent(attacker.position, Position::new(at)))
        }
    }
}
