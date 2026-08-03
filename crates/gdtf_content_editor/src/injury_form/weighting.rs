//! The draft holds the sim's own authored [`InjuryWeighting`] record (category +
use bevy::prelude::*;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryTables, InjuryWeighting},
    severity::Severity,
};

use super::draft::AutoloadState;

#[derive(Resource, Clone, PartialEq, Eq, Debug)]
pub struct WeightingDraft {
        weighting: InjuryWeighting,
        autoload:  AutoloadState,
}

impl WeightingDraft {
                #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

            pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

                                            pub fn load_table(
        &mut self,
        category: InjuryCategory,
        context: DamageContext,
        tables: &InjuryTables,
    ) {
        let bucket = |severity: Severity| {
            tables
                .table_for_category(category, context, severity)
                .map(|table| table.to_vec())
                .unwrap_or_default()
        };
        self.weighting = InjuryWeighting {
            category,
            context,
            minor: bucket(Severity::Minor),
            major: bucket(Severity::Major),
            critical: bucket(Severity::Critical),
        };
        self.autoload = AutoloadState::Done;
    }

        #[must_use]
    pub const fn category(&self) -> InjuryCategory {
        self.weighting.category
    }

            #[must_use]
    pub const fn context(&self) -> DamageContext {
        self.weighting.context
    }

            #[must_use]
    pub const fn weighting(&self) -> &InjuryWeighting {
        &self.weighting
    }

                #[must_use]
    pub const fn weighting_mut(&mut self) -> &mut InjuryWeighting {
        &mut self.weighting
    }
}

impl Default for WeightingDraft {
                    fn default() -> Self {
        Self {
            weighting: InjuryWeighting {
                category: InjuryCategory::Head,
                context:  DamageContext::Ranged,
                minor:    Vec::new(),
                major:    Vec::new(),
                critical: Vec::new(),
            },
            autoload:  AutoloadState::Pending,
        }
    }
}
