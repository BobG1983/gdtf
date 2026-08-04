//! Field catalog and live placements on the map.

use bevy::{
    platform::collections::HashMap,
    prelude::{Deref, Resource},
};
use serde::{Deserialize, Serialize};

use super::FieldDef;
use crate::{
    effects::fields::{ApplyFieldEffect, FieldEffect, FieldExpired, FieldTurns},
    metric::CellLevel,
    registry::Registry,
};

/// Content key for a field definition.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct FieldKey(String);

impl FieldKey {
    /// Wrap a key string.
    #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }
}

/// Catalog of field definitions.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldDefRegistry(Registry<FieldKey, FieldDef>);

impl FieldDefRegistry {
    /// From key/def pairs.
    #[must_use]
    pub fn new(defs: impl IntoIterator<Item = (FieldKey, FieldDef)>) -> Self {
        Self(Registry::new(defs))
    }

    /// Insert or replace a def.
    pub fn insert(&mut self, key: FieldKey, def: FieldDef) -> Option<FieldDef> {
        self.0.insert(key, def)
    }

    /// Look up a def.
    #[must_use]
    pub fn def(&self, key: &FieldKey) -> Option<&FieldDef> {
        self.0.get(key)
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A field currently on the map, with remaining lifetime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedField {
    def:       FieldDef,
    remaining: Option<FieldTurns>,
}

impl PlacedField {
    /// Place from a def, seeding the countdown.
    #[must_use]
    pub fn from_def(def: FieldDef) -> Self {
        let remaining = FieldEffect::consequences_of(&def)
            .iter()
            .map(ApplyFieldEffect::initial_countdown)
            .max()
            .flatten();
        Self { def, remaining }
    }

    /// Underlying definition.
    #[must_use]
    pub const fn def(&self) -> &FieldDef {
        &self.def
    }

    /// Remaining turns, if finite.
    #[must_use]
    pub const fn remaining(&self) -> Option<FieldTurns> {
        self.remaining
    }

    /// Count down one turn; return whether expired.
    pub fn tick_down(&mut self) -> FieldExpired {
        FieldExpired::new(
            FieldEffect::consequences_of(&self.def)
                .iter()
                .any(|consequence| *consequence.count_down_one_turn(&mut self.remaining)),
        )
    }
}

/// Live fields keyed by cell.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldRegistry(HashMap<CellLevel, PlacedField>);

impl FieldRegistry {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Place or replace a field at a cell.
    pub fn spawn(&mut self, at: CellLevel, def: FieldDef) -> Option<PlacedField> {
        self.0.insert(at, PlacedField::from_def(def))
    }

    /// Field at a cell, if any.
    #[must_use]
    pub fn field_at(&self, at: &CellLevel) -> Option<&PlacedField> {
        self.0.get(at)
    }

    /// Iterate all placements.
    pub fn iter(&self) -> impl Iterator<Item = (&CellLevel, &PlacedField)> {
        self.0.iter()
    }

    /// Count down every field and drop expired ones.
    pub fn tick_down_and_expire(&mut self) {
        self.0.retain(|_cell, placed| !*placed.tick_down());
    }

    /// Number of placements.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> IntoIterator for &'a FieldRegistry {
    type Item = (&'a CellLevel, &'a PlacedField);
    type IntoIter = bevy::platform::collections::hash_map::Iter<'a, CellLevel, PlacedField>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}
