//! Emplacement occupancy and mounted-weapon components.

use bevy::prelude::{Component, Deref, Entity};

use crate::{metric::CellLevel, terrain::facing::TerrainFacing, weapon::WeaponName};

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
    /// Wrap a manned flag.
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

/// The cell the occupant stood on before it entered, held while the seat is occupied.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnteredFrom(CellLevel);

impl EnteredFrom {
    /// Wrap the cell the occupant entered from.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self(at)
    }
}

/// Weapon key for the mounted weapon spawned when occupied.
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MountedWeaponKey(WeaponName);

impl MountedWeaponKey {
    /// Wrap a weapon name.
    #[must_use]
    pub const fn new(key: WeaponName) -> Self {
        Self(key)
    }
}

/// Sides the emplacement can be entered from, as its def authored them.
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EmplacementEntrySides(Vec<TerrainFacing>);

impl EmplacementEntrySides {
    /// Wrap the def's unrotated sides.
    #[must_use]
    pub const fn new(sides: Vec<TerrainFacing>) -> Self {
        Self(sides)
    }
}

/// Which way this emplacement is turned on the map.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmplacementFacing(TerrainFacing);

impl EmplacementFacing {
    /// Wrap the placed piece's facing.
    #[must_use]
    pub const fn new(facing: TerrainFacing) -> Self {
        Self(facing)
    }
}

/// Entity of the currently spawned mounted weapon.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MountedWeaponEntity(Entity);

impl MountedWeaponEntity {
    /// Wrap the mounted weapon entity.
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }
}
