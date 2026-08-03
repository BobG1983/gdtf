//! Theme UUID and legacy name types.

use bevy::{asset::uuid::Uuid, prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

/// Stable theme identity.
///
/// `#[serde(transparent)]` round-trips it as the bare `Uuid` wire form.
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize, TypePath,
)]
#[serde(transparent)]
pub struct ThemeUuid(Uuid);

impl ThemeUuid {
    /// Wrap a UUID.
    #[must_use]
    pub const fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Nil UUID (unset).
    #[must_use]
    pub const fn nil() -> Self {
        Self(Uuid::nil())
    }

    /// Whether this is the nil key.
    #[must_use]
    pub const fn is_nil(&self) -> crate::terrain::def::NilKey {
        crate::terrain::def::NilKey::new(self.0.is_nil())
    }

    /// Fresh random theme UUID.
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Deterministic UUID derived from a legacy theme name.
    #[must_use]
    pub fn from_legacy_theme(name: &ThemeName) -> Self {
        Self(Uuid::from_u128(*crate::terrain::def::fnv1a64_u128(
            name.as_bytes(),
        )))
    }
}

/// Legacy string theme name (pre-UUID content).
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct ThemeName(String);

impl ThemeName {
    /// Wrap a name string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}
