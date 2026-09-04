//! Runtime registry of UUID theme definitions.

use bevy::prelude::Resource;

use super::{ThemeUuid, UuidThemeDef};
use crate::{registry::Registry, terrain::def::TerrainUuid};

/// Map of theme UUID to definition.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct UuidThemeRegistry(Registry<ThemeUuid, UuidThemeDef>);

impl UuidThemeRegistry {
    /// Build a registry from key/def pairs.
    #[must_use]
    pub fn new(defs: impl IntoIterator<Item = (ThemeUuid, UuidThemeDef)>) -> Self {
        Self(Registry::new(defs))
    }

    /// Insert or replace a definition. Returns the previous value if any.
    pub fn insert(&mut self, key: ThemeUuid, def: UuidThemeDef) -> Option<UuidThemeDef> {
        self.0.insert(key, def)
    }

    /// Take the definition under `key` out; answers it if it was there.
    pub fn remove(&mut self, key: &ThemeUuid) -> Option<UuidThemeDef> {
        self.0.remove(key)
    }

    /// Look up a definition by key.
    #[must_use]
    pub fn def(&self, key: &ThemeUuid) -> Option<&UuidThemeDef> {
        self.0.get(key)
    }

    /// Default floor for a theme, if present.
    #[must_use]
    pub fn default_floor(&self, key: &ThemeUuid) -> Option<TerrainUuid> {
        self.0.get(key).map(|def| def.default_floor)
    }

    /// Terrain list for a theme, if present.
    #[must_use]
    pub fn terrain(&self, key: &ThemeUuid) -> Option<&[TerrainUuid]> {
        self.0.get(key).map(|def| def.terrain.as_slice())
    }

    /// Iterate all definitions.
    pub fn defs(&self) -> impl Iterator<Item = (&ThemeUuid, &UuidThemeDef)> {
        self.0.iter()
    }

    /// Number of themes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
