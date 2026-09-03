//! Lookup table of authored injury definitions.

use bevy::prelude::Resource;

use super::{InjuryDef, InjuryName};
use crate::registry::Registry;

/// Named injury definitions loaded from content.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct InjuryRegistry(Registry<InjuryName, InjuryDef>);

impl InjuryRegistry {
    /// Build from a list of name/def pairs.
    #[must_use]
    pub fn new(injuries: impl IntoIterator<Item = (InjuryName, InjuryDef)>) -> Self {
        Self(Registry::new(injuries))
    }

    /// Insert or replace a definition.
    pub fn insert(&mut self, name: InjuryName, def: InjuryDef) -> Option<InjuryDef> {
        self.0.insert(name, def)
    }

    /// Take the definition under `name` out; answers it if it was there.
    pub fn remove(&mut self, name: &InjuryName) -> Option<InjuryDef> {
        self.0.remove(name)
    }

    /// Look up a definition by name.
    #[must_use]
    pub fn def(&self, name: &InjuryName) -> Option<&InjuryDef> {
        self.0.get(name)
    }

    /// Whether a name is present.
    #[must_use]
    pub fn contains(&self, name: &InjuryName) -> bool {
        self.0.contains(name)
    }

    /// Iterate all entries.
    pub fn iter(&self) -> impl Iterator<Item = (&InjuryName, &InjuryDef)> {
        self.0.iter()
    }

    /// Number of definitions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True when empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
