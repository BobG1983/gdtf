//! Spawn bundle and damage/handling profiles for a ranged weapon.

use bevy::prelude::Bundle;

use super::{
    Accuracy, BaseSpread, DamageType, DotProfile, FatalBias, FireMode, Handedness, Kickback, Shove,
    Stable, TrajectoryStyle, Weapon, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
};
use crate::{effects::attachments::WeaponBraceBonus, magazine::Magazine};

/// Borrowed view of live weapon stats (including optional brace/DOT).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponStats<'a> {
    /// Base spread.
    pub base_spread: &'a BaseSpread,
    /// Accuracy.
    pub accuracy: &'a Accuracy,
    /// Kickback.
    pub kickback: &'a Kickback,
    /// Fatal bias.
    pub fatal_bias: &'a FatalBias,
    /// Damage.
    pub damage: &'a WeaponDamage,
    /// Punch.
    pub punch: &'a WeaponPunch,
    /// Shred.
    pub shred: &'a WeaponShred,
    /// Damage type.
    pub damage_type: &'a DamageType,
    /// Stability.
    pub stable: &'a Stable,
    /// Optional brace bonus from attachments.
    pub brace_bonus: Option<&'a WeaponBraceBonus>,
    /// Optional DOT profile.
    pub dot: Option<&'a DotProfile>,
}

/// Full component set for a spawned ranged weapon.
#[derive(Bundle, Debug, Clone, PartialEq)]
pub struct WeaponBundle {
    /// Marker.
    pub marker: Weapon,
    /// Name.
    pub name: WeaponName,
    /// Base spread.
    pub base_spread: BaseSpread,
    /// Accuracy.
    pub accuracy: Accuracy,
    /// Kickback.
    pub kickback: Kickback,
    /// Fatal bias.
    pub fatal_bias: FatalBias,
    /// Damage.
    pub damage: WeaponDamage,
    /// Punch.
    pub punch: WeaponPunch,
    /// Shred.
    pub shred: WeaponShred,
    /// Damage type.
    pub damage_type: DamageType,
    /// Magazine.
    pub magazine: Magazine,
    /// Fire modes.
    pub fire_mode: FireMode,
    /// Stability.
    pub stable: Stable,
    /// Shove tag.
    pub shove: Shove,
    /// Handedness.
    pub handedness: Handedness,
    /// Trajectory style.
    pub trajectory: TrajectoryStyle,
}

/// Damage numbers group used when building a bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DamageProfile {
    /// Damage.
    pub damage: WeaponDamage,
    /// Punch.
    pub punch: WeaponPunch,
    /// Shred.
    pub shred: WeaponShred,
    /// Damage type.
    pub damage_type: DamageType,
}

impl DamageProfile {
    /// Build a damage profile.
    #[must_use]
    pub const fn new(
        damage: WeaponDamage,
        punch: WeaponPunch,
        shred: WeaponShred,
        damage_type: DamageType,
    ) -> Self {
        Self {
            damage,
            punch,
            shred,
            damage_type,
        }
    }
}

/// Magazine / modes / handling group used when building a bundle.
#[derive(Debug, Clone, PartialEq)]
pub struct HandlingProfile {
    /// Magazine.
    pub magazine: Magazine,
    /// Fire modes.
    pub fire_mode: FireMode,
    /// Stability.
    pub stable: Stable,
    /// Shove.
    pub shove: Shove,
    /// Handedness.
    pub handedness: Handedness,
    /// Trajectory.
    pub trajectory: TrajectoryStyle,
}

impl HandlingProfile {
    /// Build with straight trajectory.
    #[must_use]
    pub const fn new(
        magazine: Magazine,
        fire_mode: FireMode,
        stable: Stable,
        shove: Shove,
        handedness: Handedness,
    ) -> Self {
        Self {
            magazine,
            fire_mode,
            stable,
            shove,
            handedness,
            trajectory: TrajectoryStyle::Straight,
        }
    }

    /// Override trajectory.
    #[must_use]
    pub const fn with_trajectory(mut self, trajectory: TrajectoryStyle) -> Self {
        self.trajectory = trajectory;
        self
    }
}

impl WeaponBundle {
    /// Assemble from name, ballistics, damage, and handling.
    #[must_use]
    pub fn new(
        name: WeaponName,
        base_spread: BaseSpread,
        accuracy: Accuracy,
        kickback: Kickback,
        fatal_bias: FatalBias,
        damage: DamageProfile,
        handling: HandlingProfile,
    ) -> Self {
        Self {
            marker: Weapon,
            name,
            base_spread,
            accuracy,
            kickback,
            fatal_bias,
            damage: damage.damage,
            punch: damage.punch,
            shred: damage.shred,
            damage_type: damage.damage_type,
            magazine: handling.magazine,
            fire_mode: handling.fire_mode,
            stable: handling.stable,
            shove: handling.shove,
            handedness: handling.handedness,
            trajectory: handling.trajectory,
        }
    }

    /// Borrowed stats without brace/DOT (filled by systems later).
    #[must_use]
    pub const fn stats(&self) -> WeaponStats<'_> {
        WeaponStats {
            base_spread: &self.base_spread,
            accuracy: &self.accuracy,
            kickback: &self.kickback,
            fatal_bias: &self.fatal_bias,
            damage: &self.damage,
            punch: &self.punch,
            shred: &self.shred,
            damage_type: &self.damage_type,
            stable: &self.stable,
            brace_bonus: None,
            dot: None,
        }
    }
}
