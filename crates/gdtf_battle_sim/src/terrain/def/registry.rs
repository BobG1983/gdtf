//! The **unified terrain-definition registry** — the [`TerrainUuid`]→[`TerrainDef`]
//! map (GTW-484), the sole terrain registry after GTW-496.

use bevy::{platform::collections::HashMap, prelude::Resource};

use super::{TerrainDef, TerrainUuid};

/// The **unified terrain-definition registry** — a [`TerrainUuid`]→[`TerrainDef`]
/// map (GTW-484), keyed by the stable terrain UUID.
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`TerrainUuid`]`,
/// `[`TerrainDef`]`>` (no-bare-types: a registry is a domain value, not a bare
/// `HashMap`). The sim OWNS the terrain model, so the type lives here. It holds the
/// definitions BY VALUE ([`TerrainDef`] is `Clone`), so they survive even if the
/// source asset handle is dropped.
///
/// Private inner with small accessors (a registry answers a terrain LOOKUP, not a
/// raw-map question — so no derived [`Deref`](bevy::prelude::Deref), the
/// `WeaponRegistry` / `ArmorRegistry` pattern).
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct TerrainDefRegistry(HashMap<TerrainUuid, TerrainDef>);

impl TerrainDefRegistry {
    /// Build a registry from a `(key, def)` iterator — the shape a loader (and tests)
    /// key by [`TerrainUuid`].
    #[must_use]
    pub fn new(defs: impl IntoIterator<Item = (TerrainUuid, TerrainDef)>) -> Self {
        Self(defs.into_iter().collect())
    }

    /// Insert one definition under its [`TerrainUuid`] key, returning the previous
    /// definition at that key (if any) — the per-definition insert a loader calls.
    pub fn insert(&mut self, key: TerrainUuid, def: TerrainDef) -> Option<TerrainDef> {
        self.0.insert(key, def)
    }

    /// Look up the [`TerrainDef`] for a [`TerrainUuid`] key, or [`None`] if no
    /// definition with that key is registered — the resolution a consumer reads.
    #[must_use]
    pub fn def(&self, key: &TerrainUuid) -> Option<&TerrainDef> {
        self.0.get(key)
    }

    /// An ENUMERABLE iterator over every `(key, def)` the registry holds — so a consumer
    /// can list the whole loaded terrain LIBRARY (the GTW-475 theme-authoring multi-select
    /// picks from this), mirroring
    /// [`UuidThemeRegistry::defs`](crate::level::UuidThemeRegistry::defs).
    pub fn defs(&self) -> impl Iterator<Item = (&TerrainUuid, &TerrainDef)> {
        self.0.iter()
    }

    /// How many definitions the registry holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry holds no definitions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
