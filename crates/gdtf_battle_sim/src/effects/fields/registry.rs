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

/// [`Deref`]; `#[serde(transparent)]` parses a bare RON string. [`Serialize`] too, so a
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct FieldKey(String);

impl FieldKey {
        #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldDefRegistry(Registry<FieldKey, FieldDef>);

impl FieldDefRegistry {
            #[must_use]
    pub fn new(defs: impl IntoIterator<Item = (FieldKey, FieldDef)>) -> Self {
        Self(Registry::new(defs))
    }

            pub fn insert(&mut self, key: FieldKey, def: FieldDef) -> Option<FieldDef> {
        self.0.insert(key, def)
    }

            #[must_use]
    pub fn def(&self, key: &FieldKey) -> Option<&FieldDef> {
        self.0.get(key)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedField {
        def:       FieldDef,
                remaining: Option<FieldTurns>,
}

impl PlacedField {
                        #[must_use]
    pub fn from_def(def: FieldDef) -> Self {
        let remaining = FieldEffect::consequences_of(&def)
            .iter()
            .map(ApplyFieldEffect::initial_countdown)
            .max()
            .flatten();
        Self { def, remaining }
    }

            #[must_use]
    pub const fn def(&self) -> &FieldDef {
        &self.def
    }

            #[must_use]
    pub const fn remaining(&self) -> Option<FieldTurns> {
        self.remaining
    }

                            pub fn tick_down(&mut self) -> FieldExpired {
        FieldExpired::new(
            FieldEffect::consequences_of(&self.def)
                .iter()
                .any(|consequence| *consequence.count_down_one_turn(&mut self.remaining)),
        )
    }
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldRegistry(HashMap<CellLevel, PlacedField>);

impl FieldRegistry {
            #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

                                pub fn spawn(&mut self, at: CellLevel, def: FieldDef) -> Option<PlacedField> {
        self.0.insert(at, PlacedField::from_def(def))
    }

        #[must_use]
    pub fn field_at(&self, at: &CellLevel) -> Option<&PlacedField> {
        self.0.get(at)
    }

                    pub fn iter(&self) -> impl Iterator<Item = (&CellLevel, &PlacedField)> {
        self.0.iter()
    }

                                    pub fn tick_down_and_expire(&mut self) {
        self.0.retain(|_cell, placed| !*placed.tick_down());
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

impl<'a> IntoIterator for &'a FieldRegistry {
    type Item = (&'a CellLevel, &'a PlacedField);
    type IntoIter = bevy::platform::collections::hash_map::Iter<'a, CellLevel, PlacedField>;

                    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}
