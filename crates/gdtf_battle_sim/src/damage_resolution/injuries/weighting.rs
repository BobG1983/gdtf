//! The weighted-table vocabulary — the authored per-part [`InjuryWeighting`] file,
//! its [`WeightedInjuryEntry`] rows, the [`InjuryWeight`] newtype, and the BUILT
//! per-bucket [`WeightedInjuryTable`].

use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::{DamageContext, InjuryName};
use crate::armor::InjuryCategory;

/// The **pick weight** of one injury within its `(category, severity)` bucket
/// (`docs/combat/resolution.md` injury tables) — its relative share of the
/// cumulative-weight roll (a higher weight = more likely rolled).
///
/// A no-bare-types newtype over `u32` (a weight is a domain value, not a bare
/// integer; `u32` so a bucket's summed weights never overflow a realistic roll):
/// private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON
/// number (`weight: 10`). `Serialize` is added (GTW-654) so the content editor's
/// INJURY authoring mode can write an edited weighting file back to disk
/// (behavior-inert for the sim).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(transparent)]
pub struct InjuryWeight(u32);

impl InjuryWeight {
    /// Build a pick weight from its magnitude (higher = more likely rolled).
    #[must_use]
    pub const fn new(weight: u32) -> Self {
        Self(weight)
    }
}

/// One authored **weighting row** — which injury (by its file-stem
/// [`InjuryName`] key) at what [`InjuryWeight`] (`docs/combat/resolution.md`
/// injury tables).
///
/// A row of an [`InjuryWeighting`]'s severity list and, once the loader resolves it,
/// of a built [`WeightedInjuryTable`]. The [`injury`](WeightedInjuryEntry::injury)
/// is the table-build KEY (an unknown key WARNs + is skipped at build, GTW-437), NOT
/// the display name. Both fields are typed domain values; public fields (a
/// value-object row). `Serialize` is added (GTW-654) so the content editor's INJURY
/// authoring mode can write an edited weighting file back to disk (behavior-inert
/// for the sim).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct WeightedInjuryEntry {
    /// The injury this row weights — the file-stem [`InjuryName`] key the loader
    /// resolves to an [`InjuryDef`](super::InjuryDef).
    pub injury: InjuryName,
    /// This injury's pick weight within the bucket.
    pub weight: InjuryWeight,
}

impl WeightedInjuryEntry {
    /// Build a weighting row from its injury key and pick weight.
    #[must_use]
    pub const fn new(injury: InjuryName, weight: InjuryWeight) -> Self {
        Self { injury, weight }
    }
}

/// The authored **per-`(category, context)` weighting file** — one per category per
/// wound-source [`DamageContext`], loaded from
/// `assets/content/injuries/weighting/*.weighting.ron` (`docs/combat/resolution.md`
/// injury tables; GTW-405 / GTW-452).
///
/// The de-serialization target of the `.weighting.ron` schema: the
/// [`category`](InjuryWeighting::category) + [`context`](InjuryWeighting::context) this
/// table weights, plus the three tabled severity lists (`None` graze and `Fatal` death are
/// never tabled). The same shared injury pool is weighted differently per context (GTW-452);
/// the
/// loader (GTW-437) folds these three lists, resolving keys and canonically
/// **sorting** entries, into the per-`(category, severity)` [`WeightedInjuryTable`]s —
/// so the authored Vec order never affects the deterministic roll. THIS slice only
/// names the schema. Public fields (a value-object record).
///
/// Derives [`Serialize`] too (GTW-654): the content editor's INJURY authoring mode
/// WRITES an edited weighting table back to a `.weighting.ron` through the shared
/// RON save path (the [`ArmorSpec`](crate::armor::ArmorSpec) / `GangRoster` write
/// precedent), so the authoring struct serialises to exactly the shape it
/// deserialises from.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
pub struct InjuryWeighting {
    /// The injury-pool [`InjuryCategory`] this file weights (one weighting file per
    /// `(category, context)` — both arms / both legs share one, GTW-453).
    pub category: InjuryCategory,
    /// The wound-source [`DamageContext`] this file weights (GTW-452) — the ranged /
    /// melee / fall table over the SAME shared per-category pool. `#[serde(default)]`
    /// (→ [`DamageContext::Ranged`]) so a pre-GTW-452 weighting file with no `context:`
    /// field parses as a ranged table.
    #[serde(default)]
    pub context:  DamageContext,
    /// The `Minor`-bucket weighting rows.
    pub minor:    Vec<WeightedInjuryEntry>,
    /// The `Major`-bucket weighting rows.
    pub major:    Vec<WeightedInjuryEntry>,
    /// The `Critical`-bucket weighting rows.
    pub critical: Vec<WeightedInjuryEntry>,
}

/// A BUILT, canonically-sorted weighted table for ONE `(category, severity)`
/// bucket — the resolved, roll-ready form (`docs/combat/resolution.md` injury
/// tables; GTW-405).
///
/// The loader (GTW-437) produces one of these per tabled bucket by resolving an
/// [`InjuryWeighting`]'s severity list (dropping unknown keys) and sorting the rows
/// canonically, so folder-enumeration order can never change the cumulative-weight
/// pick. The roll itself (GTW-438) reads this; THIS slice only names the type. A
/// no-bare-types newtype over `Vec<WeightedInjuryEntry>` (the bucket is a domain
/// value): private inner + derived [`Deref`] (read the rows as a slice), the only
/// construction via [`new`](WeightedInjuryTable::new).
#[derive(Deref, Debug, Clone, PartialEq, Eq, Default)]
pub struct WeightedInjuryTable(Vec<WeightedInjuryEntry>);

impl WeightedInjuryTable {
    /// Build a weighted table from its resolved, already-sorted rows (the loader
    /// owns the resolution + canonical sort, GTW-437).
    #[must_use]
    pub const fn new(entries: Vec<WeightedInjuryEntry>) -> Self {
        Self(entries)
    }
}
