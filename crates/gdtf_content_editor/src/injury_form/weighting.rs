//! Injury weighting draft resource.

use bevy::prelude::*;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryTables, InjuryWeighting},
    severity::Severity,
};

use super::draft::AutoloadState;

/// In-progress injury weighting table being authored.
#[derive(Resource, Clone, PartialEq, Eq, Debug)]
pub struct WeightingDraft {
    weighting: InjuryWeighting,
    autoload:  AutoloadState,
}

impl WeightingDraft {
    /// Whether the form should still try to autoload from the tables.
    #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

    /// Mark autoload complete.
    pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

    /// Load weighting buckets for a category and context from the tables.
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

    /// Injury category.
    #[must_use]
    pub const fn category(&self) -> InjuryCategory {
        self.weighting.category
    }

    /// Damage context.
    #[must_use]
    pub const fn context(&self) -> DamageContext {
        self.weighting.context
    }

    /// Full weighting record.
    #[must_use]
    pub const fn weighting(&self) -> &InjuryWeighting {
        &self.weighting
    }

    /// Mutable access to the weighting record.
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
