//! Runtime map of weighted injury tables keyed by category, context, and severity.

use bevy::{platform::collections::HashMap, prelude::Resource};

use super::{DamageContext, WeightedInjuryTable};
use crate::{armor::InjuryCategory, severity::Severity};

/// Lookup of weighted tables used by injury rolls.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct InjuryTables(HashMap<(InjuryCategory, DamageContext, Severity), WeightedInjuryTable>);

impl InjuryTables {
    /// Build from an iterator of keys and tables.
    #[must_use]
    pub fn new(
        tables: impl IntoIterator<
            Item = (
                (InjuryCategory, DamageContext, Severity),
                WeightedInjuryTable,
            ),
        >,
    ) -> Self {
        Self(tables.into_iter().collect())
    }

    /// Insert or replace one table.
    pub fn insert(
        &mut self,
        category: InjuryCategory,
        context: DamageContext,
        severity: Severity,
        table: WeightedInjuryTable,
    ) -> Option<WeightedInjuryTable> {
        self.0.insert((category, context, severity), table)
    }

    /// Look up the table for a body part (via its injury category).
    #[must_use]
    pub fn table(
        &self,
        part: crate::armor::BodyPart,
        context: DamageContext,
        severity: Severity,
    ) -> Option<&WeightedInjuryTable> {
        self.0.get(&(part.injury_category(), context, severity))
    }

    /// Look up by category directly.
    #[must_use]
    pub fn table_for_category(
        &self,
        category: InjuryCategory,
        context: DamageContext,
        severity: Severity,
    ) -> Option<&WeightedInjuryTable> {
        self.0.get(&(category, context, severity))
    }

    /// Number of tables.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True when empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
