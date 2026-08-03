//! The weighted-table vocabulary — the authored per-part [`InjuryWeighting`] file,
use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::{DamageContext, InjuryName};
use crate::armor::InjuryCategory;

/// private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(transparent)]
pub struct InjuryWeight(u32);

impl InjuryWeight {
        #[must_use]
    pub const fn new(weight: u32) -> Self {
        Self(weight)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct WeightedInjuryEntry {
            pub injury: InjuryName,
        pub weight: InjuryWeight,
}

impl WeightedInjuryEntry {
        #[must_use]
    pub const fn new(injury: InjuryName, weight: InjuryWeight) -> Self {
        Self { injury, weight }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
pub struct InjuryWeighting {
            pub category: InjuryCategory,
        /// melee / fall table over the SAME shared per-category pool. `#[serde(default)]`
            #[serde(default)]
    pub context:  DamageContext,
        pub minor:    Vec<WeightedInjuryEntry>,
        pub major:    Vec<WeightedInjuryEntry>,
        pub critical: Vec<WeightedInjuryEntry>,
}

#[derive(Deref, Debug, Clone, PartialEq, Eq, Default)]
pub struct WeightedInjuryTable(Vec<WeightedInjuryEntry>);

impl WeightedInjuryTable {
            #[must_use]
    pub const fn new(entries: Vec<WeightedInjuryEntry>) -> Self {
        Self(entries)
    }
}
