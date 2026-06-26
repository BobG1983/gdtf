//! The weighted-table vocabulary — the authored per-part [`InjuryWeighting`] file,
//! its [`WeightedInjuryEntry`] rows, the [`InjuryWeight`] newtype, and the BUILT
//! per-bucket [`WeightedInjuryTable`].

use bevy::{prelude::Deref, reflect::TypePath};
use serde::Deserialize;

use super::InjuryName;
use crate::armor::BodyPart;

/// The **pick weight** of one injury within its `(body_part, severity)` bucket
/// (`docs/combat/resolution.md` injury tables) — its relative share of the
/// cumulative-weight roll (a higher weight = more likely rolled).
///
/// A no-bare-types newtype over `u32` (a weight is a domain value, not a bare
/// integer; `u32` so a bucket's summed weights never overflow a realistic roll):
/// private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON
/// number (`weight: 10`).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
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
/// value-object row).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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

/// The authored **per-body-part weighting file** — one per part, loaded from
/// `assets/injuries/weighting/<part>.weighting.ron` (`docs/combat/resolution.md`
/// injury tables; GTW-405).
///
/// The de-serialization target of the `.weighting.ron` schema: the
/// [`body_part`](InjuryWeighting::body_part) this table weights, plus the three
/// tabled severity lists (`None` graze and `Fatal` death are never tabled). The
/// loader (GTW-437) folds these three lists, resolving keys and canonically
/// **sorting** entries, into the per-`(part, severity)` [`WeightedInjuryTable`]s —
/// so the authored Vec order never affects the deterministic roll. THIS slice only
/// names the schema. Public fields (a value-object record).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TypePath)]
pub struct InjuryWeighting {
    /// The body part this file weights (one weighting file per part).
    pub body_part: BodyPart,
    /// The `Minor`-bucket weighting rows.
    pub minor:     Vec<WeightedInjuryEntry>,
    /// The `Major`-bucket weighting rows.
    pub major:     Vec<WeightedInjuryEntry>,
    /// The `Critical`-bucket weighting rows.
    pub critical:  Vec<WeightedInjuryEntry>,
}

/// A BUILT, canonically-sorted weighted table for ONE `(body_part, severity)`
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
