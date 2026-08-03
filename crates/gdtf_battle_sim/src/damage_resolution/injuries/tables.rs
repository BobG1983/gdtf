use bevy::{platform::collections::HashMap, prelude::Resource};

use super::{DamageContext, WeightedInjuryTable};
use crate::{armor::InjuryCategory, severity::Severity};

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct InjuryTables(HashMap<(InjuryCategory, DamageContext, Severity), WeightedInjuryTable>);

impl InjuryTables {
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

                        pub fn insert(
        &mut self,
        category: InjuryCategory,
        context: DamageContext,
        severity: Severity,
        table: WeightedInjuryTable,
    ) -> Option<WeightedInjuryTable> {
        self.0.insert((category, context, severity), table)
    }

                                #[must_use]
    pub fn table(
        &self,
        part: crate::armor::BodyPart,
        context: DamageContext,
        severity: Severity,
    ) -> Option<&WeightedInjuryTable> {
        self.0.get(&(part.injury_category(), context, severity))
    }

                        #[must_use]
    pub fn table_for_category(
        &self,
        category: InjuryCategory,
        context: DamageContext,
        severity: Severity,
    ) -> Option<&WeightedInjuryTable> {
        self.0.get(&(category, context, severity))
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
