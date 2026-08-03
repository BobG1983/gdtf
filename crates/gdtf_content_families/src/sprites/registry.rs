use bevy::prelude::{Deref, Resource};
use gdtf_battle_sim::registry::Registry;
use serde::{Deserialize, Serialize};

use super::def::SpriteDef;

/// derived [`Deref`]; `#[serde(transparent)]` round-trips a bare RON string.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct SpriteName(String);

impl SpriteName {
        #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct SpriteDefRegistry(Registry<SpriteName, SpriteDef>);

impl SpriteDefRegistry {
            #[must_use]
    pub fn new(defs: impl IntoIterator<Item = (SpriteName, SpriteDef)>) -> Self {
        Self(Registry::new(defs))
    }

                pub fn insert(&mut self, name: SpriteName, def: SpriteDef) -> Option<SpriteDef> {
        self.0.insert(name, def)
    }

            #[must_use]
    pub fn def(&self, name: &SpriteName) -> Option<&SpriteDef> {
        self.0.get(name)
    }

            #[must_use]
    pub fn contains(&self, name: &SpriteName) -> bool {
        self.0.contains(name)
    }

                pub fn defs(&self) -> impl Iterator<Item = (&SpriteName, &SpriteDef)> {
        self.0.iter()
    }

            pub fn keys(&self) -> impl Iterator<Item = &SpriteName> {
        self.0.keys()
    }

        #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

        #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
