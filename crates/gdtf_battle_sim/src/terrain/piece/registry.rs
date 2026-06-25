//! The **terrain registry** — the name→spec map the folder loader builds (GTW-394),
//! the terrain mirror of [`WeaponRegistry`](crate::weapon::WeaponRegistry) /
//! [`ArmorRegistry`](crate::armor::ArmorRegistry).

use bevy::{platform::collections::HashMap, prelude::Resource};

use super::{TerrainName, TerrainSpec};

/// The **terrain registry** — a name→spec map the folder loader builds from
/// `assets/terrain/*.terrain.ron` (GTW-394).
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`TerrainName`]`,
/// `[`TerrainSpec`]`>` (no-bare-types: a registry is a domain value, not a bare
/// `HashMap`). The sim OWNS the terrain model, so the type lives here; the app's
/// `Load` flow POPULATES it from the loaded `assets/terrain/*.terrain.ron` folder
/// (keyed by each file's stem) and inserts it as a resource. It holds the specs
/// BY VALUE ([`TerrainSpec`] is `Clone`), so they survive even if the
/// loaded-folder asset handle is dropped.
///
/// Private inner with small accessors (the registry answers a terrain LOOKUP, not a
/// raw-map question — so no derived [`Deref`](bevy::prelude::Deref), the established
/// pattern from `WeaponRegistry` / `ArmorRegistry`).
///
/// **DORMANT after GTW-394** — nothing consumes the registry yet (binding a generated
/// cell's `TerrainName` → its spec is a downstream epic ticket). The registry is
/// seeded and the gate tests assert it is populated; consumption is a future slice.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct TerrainRegistry(HashMap<TerrainName, TerrainSpec>);

impl TerrainRegistry {
    /// Build a terrain registry from a `(name, spec)` iterator — the shape the
    /// folder loader (and tests) key by filename stem.
    #[must_use]
    pub fn new(pieces: impl IntoIterator<Item = (TerrainName, TerrainSpec)>) -> Self {
        Self(pieces.into_iter().collect())
    }

    /// Insert one terrain spec under its [`TerrainName`] key, returning the previous
    /// spec at that key (if any) — the per-file insert the folder loader calls as it
    /// iterates the loaded folder.
    pub fn insert(&mut self, name: TerrainName, spec: TerrainSpec) -> Option<TerrainSpec> {
        self.0.insert(name, spec)
    }

    /// Look up the [`TerrainSpec`] for a terrain KEY, or [`None`] if no terrain file
    /// with that stem was loaded — the setup-time resolution the battle grid reads.
    #[must_use]
    pub fn spec(&self, name: &TerrainName) -> Option<&TerrainSpec> {
        self.0.get(name)
    }

    /// How many terrain pieces the registry holds — the count the folder-load test
    /// asserts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry holds no terrain pieces.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
