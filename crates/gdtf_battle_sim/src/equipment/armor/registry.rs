use bevy::prelude::{Deref, Resource};
use serde::{Deserialize, Serialize};

use super::ArmorSpec;
use crate::registry::Registry;

/// inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON string.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArmorName(String);

impl ArmorName {
        #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// `clippy::derive_partial_eq_without_eq` lint requires it.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct ArmorRegistry(Registry<ArmorName, ArmorSpec>);

impl ArmorRegistry {
            #[must_use]
    pub fn new(armors: impl IntoIterator<Item = (ArmorName, ArmorSpec)>) -> Self {
        Self(Registry::new(armors))
    }

                pub fn insert(&mut self, name: ArmorName, spec: ArmorSpec) -> Option<ArmorSpec> {
        self.0.insert(name, spec)
    }

            #[must_use]
    pub fn spec(&self, name: &ArmorName) -> Option<&ArmorSpec> {
        self.0.get(name)
    }

            #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

        #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

                            pub fn keys(&self) -> impl Iterator<Item = &ArmorName> {
        self.0.keys()
    }

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
