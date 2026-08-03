use bevy::prelude::Resource;

use super::MeleeWeaponSpec;
use crate::{registry::Registry, weapon::WeaponName};

pub const FISTS_KEY: &str = "fists";

#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct MeleeWeaponRegistry(Registry<WeaponName, MeleeWeaponSpec>);

impl MeleeWeaponRegistry {
            #[must_use]
    pub fn new(weapons: impl IntoIterator<Item = (WeaponName, MeleeWeaponSpec)>) -> Self {
        Self(Registry::new(weapons))
    }

                pub fn insert(&mut self, name: WeaponName, spec: MeleeWeaponSpec) -> Option<MeleeWeaponSpec> {
        self.0.insert(name, spec)
    }

            #[must_use]
    pub fn spec(&self, name: &WeaponName) -> Option<&MeleeWeaponSpec> {
        self.0.get(name)
    }

                    #[must_use]
    pub fn fists(&self) -> Option<&MeleeWeaponSpec> {
        self.0.get(&WeaponName::new(FISTS_KEY.to_owned()))
    }

        #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

        #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

                                pub fn keys(&self) -> impl Iterator<Item = &WeaponName> {
        self.0.keys()
    }

                            pub fn iter(&self) -> impl Iterator<Item = (&WeaponName, &MeleeWeaponSpec)> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a MeleeWeaponRegistry {
    type Item = (&'a WeaponName, &'a MeleeWeaponSpec);
    type IntoIter = bevy::platform::collections::hash_map::Iter<'a, WeaponName, MeleeWeaponSpec>;

                    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}
