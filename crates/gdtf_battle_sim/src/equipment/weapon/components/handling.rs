use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// [`Deref`]; `#[serde(transparent)]` ([`Serialize`] so the editor's ATTACHMENT mode
/// `#[derive(Component)]` — it is the `size` LEAF of the [`crate::magazine::Magazine`]
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MagazineSize(u16);

impl MagazineSize {
        #[must_use]
    pub const fn new(rounds: u16) -> Self {
        Self(rounds)
    }

                    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// value). Private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare
/// RON string. A `#[derive(Component)]` (GTW-200) — its OWN sibling component on the
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct WeaponName(String);

impl WeaponName {
        #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// `#[derive(Component)]` newtype on the armed entity (GTW-200), parsed from the
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Handedness {
        #[default]
    OneHanded,
            TwoHanded,
}
