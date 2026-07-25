//! The INJURY-mode weighting form's **working model** (GTW-654 C2): the
//! state-scoped [`WeightingDraft`] resource the egui weighting section's controls
//! write and the save reads.
//!
//! The draft holds the sim's own authored [`InjuryWeighting`] record (category +
//! the three severity-bucket row lists), so the edited model IS the loader schema
//! (the [`InjuryDraft`](super::InjuryDraft) principle). Loading a context table
//! rebuilds the record from the BUILT
//! [`InjuryTables`](gdtf_battle_sim::injuries::InjuryTables) buckets — the resolved
//! truth of the currently-loaded content (unknown-key rows were warn-skipped at
//! build, and row order is canonical; the authored order is documented as
//! non-significant, so the reconstruction loses nothing an author owns).

use bevy::prelude::*;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryTables, InjuryWeighting},
    severity::Severity,
};

use super::draft::AutoloadState;

/// The in-progress INJURY-mode weighting DRAFT — the state-scoped resource the egui
/// weighting section writes and the save projects into the loader's authored
/// [`InjuryWeighting`] schema (GTW-654 C2). ONE `(category, context)` table is edited at
/// a time — the category combo and the per-source combo (ranged / melee / fall, GTW-452)
/// each re-load the record for the newly picked pair.
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed
/// `OnExit(Editing)` — bevy-traps #1). Private fields with named accessors /
/// mutators (no-bare-types rule 5): the table lifecycle (load-table) is owned
/// here, while row edits go through the sim record's own public bucket fields via
/// [`weighting_mut`](WeightingDraft::weighting_mut).
#[derive(Resource, Clone, PartialEq, Eq, Debug)]
pub struct WeightingDraft {
    /// The editable authored weighting record — the loader-schema record itself.
    weighting: InjuryWeighting,
    /// The one-shot open-with-a-table seed phase (see [`AutoloadState`]).
    autoload:  AutoloadState,
}

impl WeightingDraft {
    /// Whether the one-shot open-with-a-table seed is still pending — the shell
    /// checks this each Injury-mode frame and runs the autoload exactly once
    /// (idempotent under the egui multipass re-run).
    #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

    /// Mark the one-shot seed as done WITHOUT loading anything — the
    /// empty-tables branch (the def draft's empty-registry parity).
    pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

    /// Load one `(category, context)` table into the form (the category / source combos
    /// and the autoload path): the record is rebuilt from the built [`InjuryTables`]'s
    /// three severity buckets for that pair (a bucket no weighting authored loads
    /// empty). Marks the one-shot seed done.
    ///
    /// The per-source [`DamageContext`] (GTW-452) is a first-class pick: the shipped
    /// content authors one weighting file per `(category, context)`, so the authoring mode
    /// edits — and saves back to — whichever of the three source tables the author picked
    /// (a ranged pick round-trips `head.weighting.ron`, a melee pick
    /// `head.melee.weighting.ron`).
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

    /// The [`InjuryCategory`] whose table the form is editing.
    #[must_use]
    pub const fn category(&self) -> InjuryCategory {
        self.weighting.category
    }

    /// The wound-source [`DamageContext`] whose per-source table the form is editing
    /// (GTW-452) — the source combo's current pick and the save path's file-name infix.
    #[must_use]
    pub const fn context(&self) -> DamageContext {
        self.weighting.context
    }

    /// The draft's current authored weighting record (read-only — the save
    /// projection).
    #[must_use]
    pub const fn weighting(&self) -> &InjuryWeighting {
        &self.weighting
    }

    /// The weighting record, mutably — row edits (add / re-key / re-weight /
    /// remove) go through the sim record's own public bucket fields, while the
    /// table lifecycle stays owned by [`load_table`](WeightingDraft::load_table).
    #[must_use]
    pub const fn weighting_mut(&mut self) -> &mut InjuryWeighting {
        &mut self.weighting
    }
}

impl Default for WeightingDraft {
    /// The `OnEnter(Editing)` seed: an EMPTY Head-category record with the one-shot
    /// open-with-a-table autoload still `Pending` — the shell seeds it from the
    /// resolved [`InjuryTables`] on the first Injury-mode frame (the first
    /// canonical category, [`InjuryCategory::ALL`]`[0]`).
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
