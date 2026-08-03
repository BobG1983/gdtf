use bevy::{asset::uuid::Uuid, prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

/// `#[serde(transparent)]` round-trips it as the bare `Uuid` wire form (a string in RON's
/// [`Situation`](crate::situation::Situation) can use `#[serde(default)]` on its
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize, TypePath,
)]
#[serde(transparent)]
pub struct ThemeUuid(Uuid);

impl ThemeUuid {
            #[must_use]
    pub const fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

            #[must_use]
    pub const fn nil() -> Self {
        Self(Uuid::nil())
    }

        #[must_use]
    pub const fn is_nil(&self) -> crate::terrain::def::NilKey {
        crate::terrain::def::NilKey::new(self.0.is_nil())
    }

            #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

                            #[must_use]
    pub fn from_legacy_theme(name: &ThemeName) -> Self {
        Self(Uuid::from_u128(*crate::terrain::def::fnv1a64_u128(
            name.as_bytes(),
        )))
    }
}

#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct ThemeName(String);

impl ThemeName {
        #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}
