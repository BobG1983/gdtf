//! The **unified terrain-definition registry** — the [`TerrainUuid`]→[`TerrainDef`]
//! map (GTW-484), the sole terrain registry after GTW-496.

use bevy::prelude::Resource;

use super::{TerrainDef, TerrainUuid};
use crate::registry::Registry;

/// The **unified terrain-definition registry** — a [`TerrainUuid`]→[`TerrainDef`]
/// map (GTW-484), keyed by the stable terrain UUID.
///
/// A named [`Resource`] newtype over the foundation [`Registry`]`<`[`TerrainUuid`]`,
/// `[`TerrainDef`]`>` catalog map — see [`Registry`] for the shared name→def
/// surface these one-line wrappers delegate to. The sim OWNS the terrain model, so
/// the type lives here. It holds the definitions BY VALUE ([`TerrainDef`] is
/// `Clone`), so they survive even if the source asset handle is dropped.
// NOT `Eq` (GTW-547): its `TerrainDef` values carry an optional `on_death` effect whose
// `Explode` `HitType::Cone` half-angle is an `f32` (not `Eq`). `PartialEq` is enough — the
// registry is compared with `==` in tests, never keyed in a set.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct TerrainDefRegistry(Registry<TerrainUuid, TerrainDef>);

impl TerrainDefRegistry {
    /// Build a registry from a `(key, def)` iterator — the shape a loader (and tests)
    /// key by [`TerrainUuid`].
    #[must_use]
    pub fn new(defs: impl IntoIterator<Item = (TerrainUuid, TerrainDef)>) -> Self {
        Self(Registry::new(defs))
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
