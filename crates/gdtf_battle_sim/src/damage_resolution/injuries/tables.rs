//! The **injury tables** — the per-`(body_part, severity)` weighted roll tables the
//! GTW-437 loader builds from the `assets/injuries/weighting/*.weighting.ron` files
//! and the GTW-438 roll picks an injury from.

use bevy::{platform::collections::HashMap, prelude::Resource};

use super::WeightedInjuryTable;
use crate::{armor::BodyPart, severity::Severity};

/// The **injury tables** — a `(`[`BodyPart`]`, `[`Severity`]`)`→[`WeightedInjuryTable`]
/// map the GTW-437 loader builds by folding the authored per-part
/// `assets/injuries/weighting/*.weighting.ron` files, and the GTW-438 roll picks an
/// injury from on a non-graze, non-fatal wound.
///
/// A named newtype [`Resource`] over a [`HashMap`] keyed by the struck part and the
/// rolled severity (no-bare-types: the table store is a domain value, not a bare
/// `HashMap`), the sibling of the [`InjuryRegistry`](super::InjuryRegistry). Only the
/// three tabled buckets exist (`Minor` / `Major` / `Critical`); `None` (graze) and
/// `Fatal` (death via the existing gate) are never tabled, so a roll for those parts
/// finds no table.
///
/// Each [`WeightedInjuryTable`] holds its rows **canonically sorted** by the loader,
/// so folder-enumeration order can never change the cumulative-weight pick — the roll
/// is deterministic for a fixed seed regardless of OS directory order.
///
/// Private inner with small accessors (the tables answer a bucket LOOKUP, not a
/// raw-map question — so no derived [`Deref`](bevy::prelude::Deref)). Inserted
/// alongside the registry by the app's `Load` flow and held BY VALUE, so it survives
/// the loaded-folder handle being dropped on `OnExit(Load)`.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct InjuryTables(HashMap<(BodyPart, Severity), WeightedInjuryTable>);

impl InjuryTables {
    /// Build the injury tables from a `((part, severity), table)` iterator — the shape
    /// the loader produces after resolving + canonically sorting each authored bucket.
    #[must_use]
    pub fn new(
        tables: impl IntoIterator<Item = ((BodyPart, Severity), WeightedInjuryTable)>,
    ) -> Self {
        Self(tables.into_iter().collect())
    }

    /// Insert one bucket's canonically-sorted [`WeightedInjuryTable`] under its
    /// `(part, severity)` key, returning the previous table at that key (if any) —
    /// the per-bucket insert the loader calls as it folds the weighting files.
    pub fn insert(
        &mut self,
        part: BodyPart,
        severity: Severity,
        table: WeightedInjuryTable,
    ) -> Option<WeightedInjuryTable> {
        self.0.insert((part, severity), table)
    }

    /// Look up the [`WeightedInjuryTable`] for a `(part, severity)` bucket, or [`None`]
    /// if no weighting authored that bucket (an empty/missing bucket the GTW-438 roll
    /// still draws-then-discards for, keeping the RNG stream content-independent).
    #[must_use]
    pub fn table(&self, part: BodyPart, severity: Severity) -> Option<&WeightedInjuryTable> {
        self.0.get(&(part, severity))
    }

    /// How many `(part, severity)` buckets the tables hold — the count the
    /// folder-load test asserts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the tables hold no buckets.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
