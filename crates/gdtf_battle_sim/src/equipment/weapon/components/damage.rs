use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// resolution.md §6) — authored on the weapon but unused in this data slice.
/// A weapon NUMBER. Private inner + derived [`Deref`]; `#[serde(transparent)]`
/// `#[derive(Component)]` (GTW-200) — a sibling component on the armed entity.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FatalBias(f32);

impl FatalBias {
        #[must_use]
    pub const fn new(fatal_bias: f32) -> Self {
        Self(fatal_bias)
    }
}

/// Private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON
/// `#[derive(Component)]` (GTW-200) — a sibling component on the armed entity.
#[derive(
    Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct WeaponDamage(i32);

impl WeaponDamage {
        #[must_use]
    pub const fn new(damage: i32) -> Self {
        Self(damage)
    }
}

/// derived [`Deref`]; `#[serde(transparent)]` ([`Serialize`] so the editor's
/// `#[derive(Component)]` (GTW-200) — a sibling component on the armed entity.
#[derive(
    Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct WeaponPunch(i32);

impl WeaponPunch {
        #[must_use]
    pub const fn new(punch: i32) -> Self {
        Self(punch)
    }
}

/// a derived [`Deref`], and `#[serde(transparent)]` ([`Serialize`] so the editor's
/// `#[derive(Component)]` (GTW-200) — a sibling component on the armed entity.
#[derive(
    Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct WeaponShred(i32);

impl WeaponShred {
            #[must_use]
    pub const fn new(shred: i32) -> Self {
        Self(shred)
    }
}

/// its type by variant. A `#[derive(Component)]` (GTW-200) — a sibling component
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default)]
pub enum DamageType {
        Shock,
        Blast,
        Chem,
        #[default]
    Kinetic,
        Plasma,
        Rend,
        Las,
}

impl DamageType {
                    pub const ALL: [Self; 7] = [
        Self::Shock,
        Self::Blast,
        Self::Chem,
        Self::Kinetic,
        Self::Plasma,
        Self::Rend,
        Self::Las,
    ];
}
