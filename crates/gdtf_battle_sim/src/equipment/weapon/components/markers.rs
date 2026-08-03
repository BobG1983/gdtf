//! Unit markers for weapon entities.

use bevy::prelude::Component;

/// Marks a ranged weapon entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Weapon;

/// Marks a mounted (fixed) weapon entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MountedWeapon;
