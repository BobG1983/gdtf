use bevy::prelude::Resource;

use super::{ThemeUuid, UuidThemeDef};
use crate::{registry::Registry, terrain::def::TerrainUuid};

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct UuidThemeRegistry(Registry<ThemeUuid, UuidThemeDef>);

impl UuidThemeRegistry {
            #[must_use]
    pub fn new(defs: impl IntoIterator<Item = (ThemeUuid, UuidThemeDef)>) -> Self {
        Self(Registry::new(defs))
    }

            pub fn insert(&mut self, key: ThemeUuid, def: UuidThemeDef) -> Option<UuidThemeDef> {
        self.0.insert(key, def)
    }

            #[must_use]
    pub fn def(&self, key: &ThemeUuid) -> Option<&UuidThemeDef> {
        self.0.get(key)
    }

                    #[must_use]
    pub fn default_floor(&self, key: &ThemeUuid) -> Option<TerrainUuid> {
        self.0.get(key).map(|def| def.default_floor)
    }

                #[must_use]
    pub fn terrain(&self, key: &ThemeUuid) -> Option<&[TerrainUuid]> {
        self.0.get(key).map(|def| def.terrain.as_slice())
    }

            pub fn defs(&self) -> impl Iterator<Item = (&ThemeUuid, &UuidThemeDef)> {
        self.0.iter()
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
