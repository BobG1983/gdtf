//! Emplacement occupancy and mounted-weapon components.

use bevy::prelude::{Component, Deref, Entity};

use crate::weapon::WeaponName;

/// Whether the emplacement is vacant or occupied.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EmplacementState {
    /// No ganger manning it.
    #[default]
    Vacant,
    /// A ganger is manning it.
    Occupied,
}

/// Whether the emplacement is currently manned.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmplacementManned(bool);

impl EmplacementManned {
    #[must_use]
    pub const fn new(manned: bool) -> Self {
        Self(manned)
    }
}

impl EmplacementState {
    /// True when Occupied.
    #[must_use]
    pub const fn is_occupied(self) -> EmplacementManned {
        EmplacementManned::new(matches!(self, Self::Occupied))
    }
}

/// The ganger currently manning this emplacement.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmplacementOccupant(Entity);

impl EmplacementOccupant {
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

/// Weapon key for the mounted weapon spawned when occupied.
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MountedWeaponKey(WeaponName);

impl MountedWeaponKey {
    #[must_use]
    pub const fn new(key: WeaponName) -> Self {
        Self(key)
    }
}

/// Entity of the currently spawned mounted weapon.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MountedWeaponEntity(Entity);

impl MountedWeaponEntity {
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }
}
