//! Weighted injury tables authored per category and context.

use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::{DamageContext, InjuryName};
use crate::armor::InjuryCategory;

/// Relative weight of one injury in a table.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(transparent)]
pub struct InjuryWeight(u32);

impl InjuryWeight {
    /// Build from a raw weight.
    #[must_use]
    pub const fn new(weight: u32) -> Self {
        Self(weight)
    }
}

/// One row in a weighted table.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct WeightedInjuryEntry {
    /// Injury to pick.
    pub injury: InjuryName,
    /// Relative weight.
    pub weight: InjuryWeight,
}

impl WeightedInjuryEntry {
    /// Build a table row.
    #[must_use]
    pub const fn new(injury: InjuryName, weight: InjuryWeight) -> Self {
        Self { injury, weight }
    }
}

/// Authored weighting file for one injury category and damage context.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
pub struct InjuryWeighting {
    /// Body-part category.
    pub category: InjuryCategory,
    /// Damage context (defaults to Ranged when omitted).
    #[serde(default)]
    pub context: DamageContext,
    /// Minor-severity rows.
    pub minor: Vec<WeightedInjuryEntry>,
    /// Major-severity rows.
    pub major: Vec<WeightedInjuryEntry>,
    /// Critical-severity rows.
    pub critical: Vec<WeightedInjuryEntry>,
}

/// Runtime list of weighted entries used by a single roll.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Default)]
pub struct WeightedInjuryTable(Vec<WeightedInjuryEntry>);

impl WeightedInjuryTable {
    /// Build from a list of entries.
    #[must_use]
    pub const fn new(entries: Vec<WeightedInjuryEntry>) -> Self {
        Self(entries)
    }
}
