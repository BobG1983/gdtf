//! The **prefab schema** — the UUID-keyed level-fragment `.ron` format introduced by
//! the GTW-476 data-model refactor (child T04) and the SOLE prefab model after GTW-496
//! retired the legacy filename-stem-keyed prefab types.
//!
//! The schema:
//!
//! - references terrain by the stable [`TerrainUuid`](crate::terrain::def::TerrainUuid) key
//!   (the unified terrain model, GTW-484) and the theme by the stable
//!   [`ThemeUuid`](super::ThemeUuid) (GTW-485);
//! - collapses all authored geometry into ONE
//!   [`placements`](spec::PrefabSpec::placements) list of placed-UUID entries
//!   (each a [`TerrainPlacementEntry`]) — the per-piece sim/presenter behaviour lives in
//!   the referenced [`TerrainDef`](crate::terrain::def::TerrainDef), so a placement carries
//!   only *which* piece goes *where*;
//! - carries NO authored-opening field and NO per-prefab validation path — inter-fragment
//!   connectivity is by-construction in the assembler (the 1-cell `default_floor` seam every
//!   placement reserves; the old per-prefab opening machinery was removed in GTW-497), not
//!   authored per-prefab and validated fail-closed.
//!
//! GTW-486 added the SPEC TYPES ([`PrefabSpec`] / [`TerrainPlacementEntry`]); GTW-488
//! (child T05b) added the re-keyed REGISTRY ([`PrefabRegistry`] / [`PrefabKey`] /
//! [`Prefab`]) — keyed by the stable [`ThemeUuid`](super::ThemeUuid). GTW-557 dropped
//! the now-meaningless `_v2`/`V2`/`2` suffixes (the legacy v1 model was fully deleted in
//! GTW-494/496).
//!
//! Also houses the **shared prefab schema vocabulary** — [`SpawnRole`] and [`PrefabName`]
//! — previously in the sibling `prefab.rs` vocab file (merged here by GTW-557).
//!
//! Mirrors the sibling dir-module layout (memory: *code-health-module-layout*): this
//! `mod.rs` is wiring-only; per-concern files carry the types; `test` houses the unit tests.

mod placement;
mod registry;
mod spec;

#[cfg(test)]
mod test;

use bevy::prelude::Deref;
pub use placement::TerrainPlacementEntry;
pub use registry::{Prefab, PrefabKey, PrefabRegistry};
use serde::{Deserialize, Serialize};
pub use spec::PrefabSpec;

/// The **role** a prefab plays in an assembled level — a closed `PlayerEnemyFill`-style
/// set the assembler buckets prefabs by (GTW-418 C3).
///
/// A named domain enum (no-bare-types: a fragment's deployment role is a domain value,
/// not a bare `u8`/string). The assembler picks a [`Player`](SpawnRole::Player) fragment
/// for the human deployment zone, an [`Enemy`](SpawnRole::Enemy) fragment for the
/// opposing one, and [`Fill`](SpawnRole::Fill) fragments for the connective interior.
/// Derives [`Deserialize`] so a prefab `.ron` names its role as a bare variant
/// (`role: Fill`), and [`Hash`]/[`Eq`] so the [`PrefabKey`] can key on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum SpawnRole {
    /// A fragment that hosts the human player's deployment zone.
    Player,
    /// A fragment that hosts the enemy deployment zone.
    Enemy,
    /// A generic connective / interior fragment (neither deployment zone).
    Fill,
}

/// A prefab's **name** — the stable identifier the [`PrefabRegistry`] records and the
/// loader derives from each prefab file's stem (GTW-418).
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
