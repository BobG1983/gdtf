//! Prefab registry keyed by theme, size, and spawn role.

use bevy::{platform::collections::HashMap, prelude::Resource};

use super::PrefabSpec;
use crate::level::{GridSize, PrefabName, SpawnRole, ThemeUuid};

/// Named prefab with its authoring spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefab {
    name: PrefabName,
    spec: PrefabSpec,
}

impl Prefab {
    /// Build a prefab from name and spec.
    #[must_use]
    pub const fn new(name: PrefabName, spec: PrefabSpec) -> Self {
        Self { name, spec }
    }

    /// Prefab name.
    #[must_use]
    pub const fn name(&self) -> &PrefabName {
        &self.name
    }

    /// Prefab authoring spec.
    #[must_use]
    pub const fn spec(&self) -> &PrefabSpec {
        &self.spec
    }
}

/// Lookup key for prefabs that share theme, grid size, and role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrefabKey {
    /// Theme this prefab belongs to.
    pub theme: ThemeUuid,
    /// Grid size the prefab fills.
    pub size: GridSize,
    /// Spawn role (player, enemy, fill).
    pub role: SpawnRole,
}

impl PrefabKey {
    /// Build a key from theme, size, and role.
    #[must_use]
    pub const fn new(theme: ThemeUuid, size: GridSize, role: SpawnRole) -> Self {
        Self { theme, size, role }
    }
}

/// Runtime registry of prefabs grouped by [`PrefabKey`].
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct PrefabRegistry(HashMap<PrefabKey, Vec<Prefab>>);

impl PrefabRegistry {
    /// Insert a prefab under the key derived from its spec.
    pub fn insert(&mut self, prefab: Prefab) {
        let key = PrefabKey::new(prefab.spec().theme, prefab.spec().size, prefab.spec().role);
        self.0.entry(key).or_default().push(prefab);
    }

    /// Prefabs for a key, or an empty slice.
    #[must_use]
    pub fn prefabs_for(&self, key: &PrefabKey) -> &[Prefab] {
        self.0.get(key).map_or(&[], Vec::as_slice)
    }

    /// Total number of prefabs across all keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.values().map(Vec::len).sum()
    }

    /// Whether every key has an empty list.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.values().all(Vec::is_empty)
    }

    /// Iterate registry keys.
    pub fn keys(&self) -> impl Iterator<Item = &PrefabKey> {
        self.0.keys()
    }

    /// Iterate all prefabs.
    pub fn iter(&self) -> impl Iterator<Item = &Prefab> {
        self.0.values().flatten()
    }
}
