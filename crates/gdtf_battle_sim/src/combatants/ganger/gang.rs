//! Authored gang roster and registry.

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

/// Gang display name.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GangName(String);

impl GangName {
    /// Wrap a name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// One authored member of a gang roster.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GangMember {
    /// Display name.
    pub name: GangerName,
    /// Speed attribute.
    pub speed: Speed,
    /// Aim attribute.
    pub aim: Aim,
    /// Strength attribute.
    pub strength: Strength,
    /// Toughness attribute.
    pub toughness: Toughness,
    /// Reflexes attribute.
    pub reflexes: Reflexes,
    /// Cool attribute.
    pub cool: Cool,
    /// Grit attribute.
    pub grit: Grit,
    /// Luck attribute.
    pub luck: Luck,
    /// Armor loadout key.
    pub armor: ArmorName,
    /// Primary weapon key.
    pub weapon: WeaponName,
    /// Optional melee weapon key.
    #[serde(default)]
    pub melee_weapon: Option<WeaponName>,
}

impl GangMember {
    /// Attributes bundle for stat derivation.
    #[must_use]
    pub const fn attributes(&self) -> crate::ganger::GangerAttributes {
        crate::ganger::GangerAttributes {
            speed: self.speed,
            aim: self.aim,
            strength: self.strength,
            toughness: self.toughness,
            reflexes: self.reflexes,
            cool: self.cool,
            grit: self.grit,
            luck: self.luck,
        }
    }
}

/// List of members for one gang.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TypePath)]
pub struct GangRoster {
    /// Members.
    #[serde(default)]
    pub members: Vec<GangMember>,
}

impl GangRoster {
    /// Build from members.
    #[must_use]
    pub fn new(members: impl IntoIterator<Item = GangMember>) -> Self {
        Self {
            members: members.into_iter().collect(),
        }
    }

    /// Find a member by name.
    #[must_use]
    pub fn member(&self, name: &GangerName) -> Option<&GangMember> {
        self.members.iter().find(|member| &member.name == name)
    }
}

/// All loaded gangs keyed by name.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct GangRegistry(Registry<GangName, GangRoster>);

impl GangRegistry {
    /// Build from name→roster pairs.
    #[must_use]
    pub fn new(gangs: impl IntoIterator<Item = (GangName, GangRoster)>) -> Self {
        Self(Registry::new(gangs))
    }

    /// Insert or replace a roster.
    pub fn insert(&mut self, name: GangName, roster: GangRoster) -> Option<GangRoster> {
        self.0.insert(name, roster)
    }

    /// Lookup a roster by name.
    #[must_use]
    pub fn roster(&self, name: &GangName) -> Option<&GangRoster> {
        self.0.get(name)
    }

    /// Number of gangs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True when empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate gang names.
    pub fn keys(&self) -> impl Iterator<Item = &GangName> {
        self.0.keys()
    }

    /// Iterate name→roster pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&GangName, &GangRoster)> {
        self.0.iter()
    }
}
