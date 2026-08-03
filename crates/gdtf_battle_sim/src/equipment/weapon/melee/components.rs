use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// `#[serde(transparent)]` parses a bare RON scalar (the
/// `#[derive(Component)]` so it lives as a sibling component on the armed melee-weapon
/// reach-1 weapon (the `#[serde(default)]` on the spec field makes the field optional).
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Reach(u16);

impl Reach {
        /// `#[serde(default)]` melee spec field falls back to this, and it doubles as the
        pub const DEFAULT: Self = Self(1);

        #[must_use]
    pub const fn new(reach: u16) -> Self {
        Self(reach)
    }
}

impl Default for Reach {
        fn default() -> Self {
        Self::DEFAULT
    }
}

/// The **`MeleeWeapon` MARKER** — a unit `#[derive(Component)]` tag (no data) marking
/// `#[derive(Component)]` newtype on the same entity (the shared
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MeleeWeapon;
