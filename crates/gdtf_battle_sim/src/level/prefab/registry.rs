use bevy::{platform::collections::HashMap, prelude::Resource};

use super::PrefabSpec;
use crate::level::{GridSize, PrefabName, SpawnRole, ThemeUuid};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefab {
        name: PrefabName,
        spec: PrefabSpec,
}

impl Prefab {
                        #[must_use]
    pub const fn new(name: PrefabName, spec: PrefabSpec) -> Self {
        Self { name, spec }
    }

        #[must_use]
    pub const fn name(&self) -> &PrefabName {
        &self.name
    }

        #[must_use]
    pub const fn spec(&self) -> &PrefabSpec {
        &self.spec
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrefabKey {
        pub theme: ThemeUuid,
        pub size:  GridSize,
        pub role:  SpawnRole,
}

impl PrefabKey {
        #[must_use]
    pub const fn new(theme: ThemeUuid, size: GridSize, role: SpawnRole) -> Self {
        Self { theme, size, role }
    }
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct PrefabRegistry(HashMap<PrefabKey, Vec<Prefab>>);

impl PrefabRegistry {
                                pub fn insert(&mut self, prefab: Prefab) {
        let key = PrefabKey::new(prefab.spec().theme, prefab.spec().size, prefab.spec().role);
        self.0.entry(key).or_default().push(prefab);
    }

            #[must_use]
    pub fn prefabs_for(&self, key: &PrefabKey) -> &[Prefab] {
        self.0.get(key).map_or(&[], Vec::as_slice)
    }

            #[must_use]
    pub fn len(&self) -> usize {
        self.0.values().map(Vec::len).sum()
    }

        #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.values().all(Vec::is_empty)
    }

                pub fn keys(&self) -> impl Iterator<Item = &PrefabKey> {
        self.0.keys()
    }

                pub fn iter(&self) -> impl Iterator<Item = &Prefab> {
        self.0.values().flatten()
    }
}
