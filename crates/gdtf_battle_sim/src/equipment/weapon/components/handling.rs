//! Magazine size, name, and handedness.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// Magazine capacity in rounds.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MagazineSize(u16);

impl MagazineSize {
    /// Wrap a capacity.
    #[must_use]
    pub const fn new(rounds: u16) -> Self {
        Self(rounds)
    }

    /// Inner capacity.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Weapon content key / display name.
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct WeaponName(String);

impl WeaponName {
    /// Wrap a name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// One- or two-handed grip.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Handedness {
    #[default]
    OneHanded,
    TwoHanded,
}
