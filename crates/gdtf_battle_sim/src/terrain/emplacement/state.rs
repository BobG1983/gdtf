use bevy::prelude::{Component, Deref, Entity};

use crate::weapon::WeaponName;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EmplacementState {
            #[default]
    Vacant,
        Occupied,
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmplacementManned(bool);

impl EmplacementManned {
        #[must_use]
    pub const fn new(manned: bool) -> Self {
        Self(manned)
    }
}

impl EmplacementState {
            #[must_use]
    pub const fn is_occupied(self) -> EmplacementManned {
        EmplacementManned::new(matches!(self, Self::Occupied))
    }
}

#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmplacementOccupant(Entity);

impl EmplacementOccupant {
        #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MountedWeaponKey(WeaponName);

impl MountedWeaponKey {
                #[must_use]
    pub const fn new(key: WeaponName) -> Self {
        Self(key)
    }
}

#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MountedWeaponEntity(Entity);

impl MountedWeaponEntity {
        #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }
}
