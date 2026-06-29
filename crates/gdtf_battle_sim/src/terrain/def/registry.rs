//! The **unified terrain-definition registry** — the [`TerrainUuid`]→[`TerrainDef`]
//! map (GTW-484), the UUID-keyed successor to the legacy
//! [`TerrainRegistry`](crate::terrain::piece::TerrainRegistry).

use bevy::{platform::collections::HashMap, prelude::Resource};

use super::{TerrainDef, TerrainUuid};

/// The **unified terrain-definition registry** — a [`TerrainUuid`]→[`TerrainDef`]
/// map (GTW-484), mirroring the legacy
/// [`TerrainRegistry`](crate::terrain::piece::TerrainRegistry) shape but keyed by the
/// stable UUID instead of a filename stem.
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`TerrainUuid`]`,
/// `[`TerrainDef`]`>` (no-bare-types: a registry is a domain value, not a bare
/// `HashMap`). The sim OWNS the terrain model, so the type lives here. It holds the
/// definitions BY VALUE ([`TerrainDef`] is `Clone`), so they survive even if the
/// source asset handle is dropped.
///
/// Private inner with small accessors (a registry answers a terrain LOOKUP, not a
/// raw-map question — so no derived [`Deref`](bevy::prelude::Deref), the
/// `TerrainRegistry` / `WeaponRegistry` / `ArmorRegistry` pattern).
///
/// **Purely additive (GTW-484)** — nothing consumes this registry yet; no loader
/// populates it. It is exercised only by this ticket's unit tests. Wiring a loader and
/// binding consumers are downstream slices of the GTW-476 refactor.
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
