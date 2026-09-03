//! Named armor specs loaded from content.

use bevy::prelude::{Deref, Resource};
use serde::{Deserialize, Serialize};

use super::ArmorSpec;
use crate::registry::Registry;

/// Armor content key.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArmorName(String);

impl ArmorName {
    /// Wrap a name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// All loaded armor specs.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct ArmorRegistry(Registry<ArmorName, ArmorSpec>);

impl ArmorRegistry {
    /// Build from name→spec pairs.
    #[must_use]
    pub fn new(armors: impl IntoIterator<Item = (ArmorName, ArmorSpec)>) -> Self {
        Self(Registry::new(armors))
    }

    /// Insert or replace a spec.
    pub fn insert(&mut self, name: ArmorName, spec: ArmorSpec) -> Option<ArmorSpec> {
        self.0.insert(name, spec)
    }

    /// Take the spec under `name` out; answers it if it was there.
    pub fn remove(&mut self, name: &ArmorName) -> Option<ArmorSpec> {
        self.0.remove(name)
    }

    /// Lookup by name.
    #[must_use]
    pub fn spec(&self, name: &ArmorName) -> Option<&ArmorSpec> {
        self.0.get(name)
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True when empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate names.
    pub fn keys(&self) -> impl Iterator<Item = &ArmorName> {
        self.0.keys()
    }

    /// Iterate name→spec pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&ArmorName, &ArmorSpec)> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a ArmorRegistry {
    type Item = (&'a ArmorName, &'a ArmorSpec);
    type IntoIter = bevy::platform::collections::hash_map::Iter<'a, ArmorName, ArmorSpec>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}
