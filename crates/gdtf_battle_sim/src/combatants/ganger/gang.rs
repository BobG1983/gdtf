use bevy::{
    prelude::{Deref, Resource},
    reflect::TypePath,
};
use serde::{Deserialize, Serialize};

use super::{
    attributes::{Aim, Cool, Grit, Reflexes, Speed, Strength},
    vitals::{GangerName, Luck, Toughness},
};
use crate::{armor::ArmorName, registry::Registry, weapon::WeaponName};

/// [`Deref`] (house style — never a hand-written `impl Deref`). `#[serde(transparent)]`
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GangName(String);

impl GangName {
                #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GangMember {
                /// `#[serde(transparent)]`).
    pub name:         GangerName,
            /// scalar ([`Speed`] is `#[serde(transparent)]`).
    pub speed:        Speed,
                pub aim:          Aim,
            pub strength:     Strength,
                pub toughness:    Toughness,
            pub reflexes:     Reflexes,
            pub cool:         Cool,
            pub grit:         Grit,
                pub luck:         Luck,
                    pub armor:        ArmorName,
                    pub weapon:       WeaponName,
                /// [`setup_battle`](crate::situation::setup_battle). `#[serde(default)]` ⇒ an
                #[serde(default)]
    pub melee_weapon: Option<WeaponName>,
}

impl GangMember {
                    #[must_use]
    pub const fn attributes(&self) -> crate::ganger::GangerAttributes {
        crate::ganger::GangerAttributes {
            speed:     self.speed,
            aim:       self.aim,
            strength:  self.strength,
            toughness: self.toughness,
            reflexes:  self.reflexes,
            cool:      self.cool,
            grit:      self.grit,
            luck:      self.luck,
        }
    }
}

/// satisfy. `#[serde(default)]` on `members` lets an authored file omit an empty member list.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TypePath)]
pub struct GangRoster {
        /// weapon + armor keys). `#[serde(default)]` gives an empty roster for a file that
        #[serde(default)]
    pub members: Vec<GangMember>,
}

impl GangRoster {
        #[must_use]
    pub fn new(members: impl IntoIterator<Item = GangMember>) -> Self {
        Self {
            members: members.into_iter().collect(),
        }
    }

                    #[must_use]
    pub fn member(&self, name: &GangerName) -> Option<&GangMember> {
        self.members.iter().find(|member| &member.name == name)
    }
}

#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct GangRegistry(Registry<GangName, GangRoster>);

impl GangRegistry {
            #[must_use]
    pub fn new(gangs: impl IntoIterator<Item = (GangName, GangRoster)>) -> Self {
        Self(Registry::new(gangs))
    }

                pub fn insert(&mut self, name: GangName, roster: GangRoster) -> Option<GangRoster> {
        self.0.insert(name, roster)
    }

            #[must_use]
    pub fn roster(&self, name: &GangName) -> Option<&GangRoster> {
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

                            pub fn keys(&self) -> impl Iterator<Item = &GangName> {
        self.0.keys()
    }

                    pub fn iter(&self) -> impl Iterator<Item = (&GangName, &GangRoster)> {
        self.0.iter()
    }
}
