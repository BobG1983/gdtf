//! The **injury tables** — the per-`(category, context, severity)` weighted roll tables the
//! GTW-437 loader builds from the `assets/content/injuries/weighting/*.weighting.ron` files
//! (one file per `(category, context)` since GTW-452) and the GTW-438 roll picks an injury
//! from.

use bevy::{platform::collections::HashMap, prelude::Resource};

use super::{DamageContext, WeightedInjuryTable};
use crate::{armor::InjuryCategory, severity::Severity};

/// The **injury tables** — a
/// `(`[`InjuryCategory`]`, `[`DamageContext`]`, `[`Severity`]`)`→[`WeightedInjuryTable`]
/// map the GTW-437 loader builds by folding the authored per-`(category, context)`
/// `assets/content/injuries/weighting/*.weighting.ron` files, and the GTW-438 roll picks an
/// injury from on a non-graze, non-fatal wound.
///
/// A named newtype [`Resource`] over a [`HashMap`] keyed by the authored
/// [`InjuryCategory`] (GTW-440 / GTW-453), the wound-source [`DamageContext`] (ranged /
/// melee / fall, GTW-452), and the rolled severity (no-bare-types: the table store is a
/// domain value, not a bare `HashMap`), the sibling of the
/// [`InjuryRegistry`](super::InjuryRegistry). Build / insert key DIRECTLY by the authored
/// category + context — the `.weighting.ron` files now author a `category:` + `context:`
/// field (GTW-453 / GTW-452) — while the roll's [`table`](InjuryTables::table) LOOKUP
/// resolves a struck per-side [`BodyPart`](crate::armor::BodyPart) to its category at the
/// boundary
/// ([`injury_category`](crate::armor::BodyPart::injury_category)), so a wound on `LeftArm`
/// and a wound on `RightArm` both hit the SAME `Arm` bucket — the shared per-category pool
/// (GTW-440 C1 / C6). Only the three tabled buckets exist (`Minor` / `Major` /
/// `Critical`); `None` (graze) and `Fatal` (death via the existing gate) are never tabled,
/// so a roll for those finds no table.
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
pub struct InjuryTables(HashMap<(InjuryCategory, DamageContext, Severity), WeightedInjuryTable>);

impl InjuryTables {
    /// Build the injury tables from a `((category, context, severity), table)` iterator — the
    /// shape the loader produces after resolving + canonically sorting each authored bucket.
    /// The authored [`InjuryCategory`] + [`DamageContext`] key directly (GTW-453 / GTW-452);
    /// a later same-`(category, context)` entry REPLACES an earlier one (the loader authors
    /// one weighting file per `(category, context)` — the two arms / the two legs already
    /// share one).
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

    /// Insert one bucket's canonically-sorted [`WeightedInjuryTable`] under its
    /// `(category, context, severity)` key (GTW-453 / GTW-452: the authored
    /// [`InjuryCategory`] + [`DamageContext`] key directly), returning the previous table at
    /// that key (if any). The per-bucket insert the loader calls as it folds the
    /// per-`(category, context)` weighting files.
    pub fn insert(
        &mut self,
        category: InjuryCategory,
        context: DamageContext,
        severity: Severity,
        table: WeightedInjuryTable,
    ) -> Option<WeightedInjuryTable> {
        self.0.insert((category, context, severity), table)
    }

    /// Look up the [`WeightedInjuryTable`] for a struck `(part, context, severity)` bucket —
    /// the per-side `part` resolved to its [`InjuryCategory`] at THIS lookup boundary
    /// ([`injury_category`](crate::armor::BodyPart::injury_category), GTW-440 / GTW-453),
    /// so `LeftArm` and `RightArm` resolve the SAME shared `Arm` table, while the wound's
    /// [`DamageContext`] (ranged / melee / fall, GTW-452) picks the per-source table — or
    /// [`None`] if no weighting authored that bucket (an empty/missing bucket the GTW-438
    /// roll still draws-then-discards for, keeping the RNG stream content-independent).
    #[must_use]
    pub fn table(
        &self,
        part: crate::armor::BodyPart,
        context: DamageContext,
        severity: Severity,
    ) -> Option<&WeightedInjuryTable> {
        self.0.get(&(part.injury_category(), context, severity))
    }

    /// Look up the [`WeightedInjuryTable`] for a `(category, context, severity)` bucket
    /// directly by the authored [`InjuryCategory`] + [`DamageContext`] — the loader-side
    /// accessor (GTW-453 / GTW-452) used by the missing-weighting audit, which already holds
    /// a def's authored category and so needs no per-side [`BodyPart`](crate::armor::BodyPart)
    /// round-trip. [`None`] if no weighting authored that bucket.
    #[must_use]
    pub fn table_for_category(
        &self,
        category: InjuryCategory,
        context: DamageContext,
        severity: Severity,
    ) -> Option<&WeightedInjuryTable> {
        self.0.get(&(category, context, severity))
    }

    /// How many `(category, context, severity)` buckets the tables hold — the count the
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
