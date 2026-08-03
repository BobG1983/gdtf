use bevy::prelude::Resource;

use super::{InjuryDef, InjuryName};
use crate::registry::Registry;

#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct InjuryRegistry(Registry<InjuryName, InjuryDef>);

impl InjuryRegistry {
            #[must_use]
    pub fn new(injuries: impl IntoIterator<Item = (InjuryName, InjuryDef)>) -> Self {
        Self(Registry::new(injuries))
    }

                pub fn insert(&mut self, name: InjuryName, def: InjuryDef) -> Option<InjuryDef> {
        self.0.insert(name, def)
    }

            #[must_use]
    pub fn def(&self, name: &InjuryName) -> Option<&InjuryDef> {
        self.0.get(name)
    }

            #[must_use]
    pub fn contains(&self, name: &InjuryName) -> bool {
        self.0.contains(name)
    }

            pub fn iter(&self) -> impl Iterator<Item = (&InjuryName, &InjuryDef)> {
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
