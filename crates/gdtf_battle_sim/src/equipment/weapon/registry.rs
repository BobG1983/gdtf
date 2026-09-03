//! Named weapon specs loaded from content.

use bevy::prelude::Resource;

use super::{WeaponName, WeaponSpec};
use crate::registry::Registry;

/// All loaded ranged weapon specs.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct WeaponRegistry(Registry<WeaponName, WeaponSpec>);

impl WeaponRegistry {
    /// Build from name→spec pairs.
    #[must_use]
    pub fn new(weapons: impl IntoIterator<Item = (WeaponName, WeaponSpec)>) -> Self {
        Self(Registry::new(weapons))
    }

    /// Insert or replace a spec.
    pub fn insert(&mut self, name: WeaponName, spec: WeaponSpec) -> Option<WeaponSpec> {
        self.0.insert(name, spec)
    }

    /// Take the spec for `name` out; returns it if it was there.
    pub fn remove(&mut self, name: &WeaponName) -> Option<WeaponSpec> {
        self.0.remove(name)
    }

    /// Lookup by name.
    #[must_use]
    pub fn spec(&self, name: &WeaponName) -> Option<&WeaponSpec> {
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
    pub fn keys(&self) -> impl Iterator<Item = &WeaponName> {
        self.0.keys()
    }

    /// Iterate name→spec pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&WeaponName, &WeaponSpec)> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a WeaponRegistry {
    type Item = (&'a WeaponName, &'a WeaponSpec);
    type IntoIter = bevy::platform::collections::hash_map::Iter<'a, WeaponName, WeaponSpec>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}
