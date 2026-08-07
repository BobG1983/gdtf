//! Attacker snapshot and RNG handles for a melee resolution.

use bevy::prelude::Entity;

use super::cost::MeleeAttacker;
use crate::{
    ganger::{Facing, Faction, Fight, Luck, Position, Stance, Toughness, Tu},
    melee::MeleeWeaponHit,
    rng::{FightRng, InjuryRng, SeverityRng, ShotRng},
    weapon::DamageType,
};

/// The defender's toughness and luck after injury modifiers.
#[derive(Debug, Clone, Copy)]
pub(super) struct DefenderResilience {
    pub(super) toughness: Toughness,
    pub(super) luck:      Luck,
}

/// Frozen attacker state used during one melee resolution.
pub(super) struct AttackerSnapshot<'a> {
    pub(super) entity:             Entity,
    pub(super) position:           Position,
    pub(super) stance:             Stance,
    pub(super) facing:             Facing,
    pub(super) fight:              Fight,
    pub(super) faction:            Faction,
    pub(super) luck:               Luck,
    pub(super) weapon:             MeleeWeaponHit<'a>,
    pub(super) tu_cost:            Tu,
    pub(super) strike_damage_type: DamageType,
    pub(super) shove:              crate::weapon::Shove,
}

impl AttackerSnapshot<'_> {
    // Position and faction the reach check needs.
    pub(super) const fn reach(&self) -> MeleeAttacker {
        MeleeAttacker::new(self.position, self.faction)
    }
}

/// Mutable RNGs for the melee contest and hit fold.
pub(super) struct MeleeStreams<'a> {
    pub(super) fight:    &'a mut FightRng,
    pub(super) shot:     &'a mut ShotRng,
    pub(super) severity: &'a mut SeverityRng,
    pub(super) injury:   &'a mut InjuryRng,
}
