//! Stable UUID keys for terrain pieces.

use bevy::{asset::uuid::Uuid, prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

use crate::terrain::piece::TerrainName;

/// Content key for a terrain piece definition.
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize, TypePath,
)]
#[serde(transparent)]
pub struct TerrainUuid(Uuid);

impl TerrainUuid {
    /// Wrap a UUID.
    #[must_use]
    pub const fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Nil sentinel (no piece).
    #[must_use]
    pub const fn nil() -> Self {
        Self(Uuid::nil())
    }

    /// Whether this is the nil sentinel.
    #[must_use]
    pub const fn is_nil(&self) -> NilKey {
        NilKey::new(self.0.is_nil())
    }

    /// Fresh random key.
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Deterministic key from a legacy string name.
    #[must_use]
    pub fn from_legacy_name(name: &TerrainName) -> Self {
        Self(Uuid::from_u128(*fnv1a64_u128(name.as_bytes())))
    }
}

/// Whether a terrain key is the nil sentinel.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NilKey(bool);

impl NilKey {
    /// Wrap the flag.
    #[must_use]
    pub const fn new(is_nil: bool) -> Self {
        Self(is_nil)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LegacyKeyDigest(u128);

impl LegacyKeyDigest {
    #[must_use]
    pub(crate) const fn new(digest: u128) -> Self {
        Self(digest)
    }
}

pub(crate) fn fnv1a64_u128(bytes: &[u8]) -> LegacyKeyDigest {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let hash_pass = |iter: &mut dyn Iterator<Item = &u8>| {
        let mut hash = FNV_OFFSET;
        for &b in iter {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    };
    let high = hash_pass(&mut bytes.iter());
    let low = hash_pass(&mut bytes.iter().rev());
    LegacyKeyDigest::new((u128::from(high) << 64) | u128::from(low))
}
