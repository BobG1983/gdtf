//! The shared **prefab schema vocabulary** still referenced by the UUID-keyed
//! [`prefab_v2`](super::prefab_v2) model after the GTW-496 deletion of the legacy
//! filename-stem-keyed prefab spec / registry / key / validated-prefab / per-piece types.
//!
//! What survives here is the schema vocabulary the v2 model imports from
//! `crate::level`:
//!
//! - [`SpawnRole`] — the closed deployment role a fragment plays (player / enemy / fill),
//!   the third enumeration key of [`PrefabKey2`](super::PrefabKey2).
//! - [`PrefabName`] — the stable fragment identifier the loader derives from each prefab
//!   file's stem.
//! - [`EdgeOpening`] / [`EdgeOpeningDef`] / [`PrefabLoadError`] — the seam / connectivity
//!   types (GTW-497-bound; not removed here).
//!
//! # The edge-opening encoding
//!
//! An [`EdgeOpening`] is a walkable cell on a footprint boundary edge through which an
//! inter-prefab seam connects. The typed [`PrefabLoadError::NoEdgeOpening`] is the
//! fail-closed rejection of a fragment that authors zero of them (no panic — fail-closed
//! exclusion + a warn).

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use crate::metric::CellLevel;

/// The **role** a prefab plays in an assembled level — a closed `PlayerEnemyFill`-style
/// set the assembler buckets prefabs by (GTW-418 C3).
///
/// A named domain enum (no-bare-types: a fragment's deployment role is a domain value,
/// not a bare `u8`/string). The assembler picks a [`Player`](SpawnRole::Player) fragment
/// for the human deployment zone, an [`Enemy`](SpawnRole::Enemy) fragment for the
/// opposing one, and [`Fill`](SpawnRole::Fill) fragments for the connective interior.
/// Derives [`Deserialize`] so a prefab `.ron` names its role as a bare variant
/// (`role: Fill`), and [`Hash`]/[`Eq`] so the [`PrefabKey2`](super::PrefabKey2) can key
/// on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum SpawnRole {
    /// A fragment that hosts the human player's deployment zone.
    Player,
    /// A fragment that hosts the enemy deployment zone.
    Enemy,
    /// A generic connective / interior fragment (neither deployment zone).
    Fill,
}

/// A prefab's **name** — the stable identifier the [`PrefabRegistry2`](super::PrefabRegistry2)
/// records and the loader derives from each prefab file's stem (GTW-418).
///
/// A key newtype over [`String`] (no-bare-types rule 1), the
/// [`WeaponName`](crate::weapon::WeaponName) precedent. Private inner + derived
/// [`Deref`]; `#[serde(transparent)]` is NOT needed (a prefab file does not author its
/// own name — the loader keys it by the file stem), but the type still derives the
/// hashing traits so it can index a registry entry.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PrefabName(String);

impl PrefabName {
    /// Build a prefab name from its identifier string (the loader passes the file stem).
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// One **edge opening** — a walkable cell on a prefab footprint's boundary edge through
/// which the inter-prefab 1-cell `default_floor` seam connects (GTW-418 C6).
///
/// A named newtype over a [`CellLevel`] (no-bare-types: a seam doorway is a domain value,
/// not a bare grid key) so an opening is never confused with an arbitrary authored cell.
/// The assembler joins a neighbouring fragment's seam to this cell; a prefab with ZERO
/// openings cannot connect and is rejected at load ([`PrefabLoadError::NoEdgeOpening`]).
/// Private inner + derived [`Deref`]; deserializes through the [`CellLevel`]
/// `(cell, level)` authoring shape via the [`EdgeOpeningDef`] intermediate, and
/// serializes back out through the SAME shape (`#[serde(into = "EdgeOpeningDef")]`),
/// so the editor prefab saver (GTW-432) writes an opening that round-trips byte-for-byte
/// through the `from = "EdgeOpeningDef"` reader.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(from = "EdgeOpeningDef", into = "EdgeOpeningDef")]
pub struct EdgeOpening(CellLevel);

impl EdgeOpening {
    /// Build an edge opening at a `(cell, level)` boundary cell.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self(at)
    }

    /// The `(cell, level)` boundary cell this opening sits on.
    #[must_use]
    pub const fn at(self) -> CellLevel {
        self.0
    }
}

/// The authored RON shape an [`EdgeOpening`] deserializes from — a single `at`
/// `(cell, level)`, routed through [`EdgeOpening::new`] (GTW-418).
///
/// A serde intermediate (the [`CellLevelDef`](crate::metric) precedent) so an authored
/// opening writes `(at: (cell: .., level: ..))` and keeps the newtype inner private
/// across (de)serialization — read in through `from`, written back out through `into`.
#[derive(Deserialize, Serialize)]
pub struct EdgeOpeningDef {
    /// The `(cell, level)` boundary cell the opening sits on.
    at: CellLevel,
}

impl From<EdgeOpeningDef> for EdgeOpening {
    fn from(def: EdgeOpeningDef) -> Self {
        Self::new(def.at)
    }
}

impl From<EdgeOpening> for EdgeOpeningDef {
    fn from(opening: EdgeOpening) -> Self {
        Self { at: opening.at() }
    }
}

/// Why a prefab `.ron` was rejected at load — the fail-closed, no-panic error of the
/// prefab validation (GTW-418 C6).
///
/// The handled rejection reason (no-bare-types: the failure is a domain value, not a bare
/// `()`/`bool`; the no-panic contract — the loader excludes the prefab + warns rather than
/// `unwrap`/`panic`). The [`GridSizeError`](super::GridSizeError) precedent: each variant
/// names what was wrong so a caller can log exactly which prefab failed and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrefabLoadError {
    /// The prefab authored ZERO [`EdgeOpening`]s, so the inter-prefab seam cannot connect
    /// to it — connectivity-by-construction violated (C6). Names the rejected prefab.
    NoEdgeOpening(PrefabName),
}

impl std::fmt::Display for PrefabLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoEdgeOpening(name) => write!(
                f,
                "prefab `{}` has zero edge openings; every prefab must author at least one \
                 walkable boundary-edge cell for the inter-prefab seam to connect to",
                **name,
            ),
        }
    }
}

impl std::error::Error for PrefabLoadError {}
