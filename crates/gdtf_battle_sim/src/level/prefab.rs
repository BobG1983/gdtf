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
//!
//! GTW-497 (the FINAL child of the GTW-476 data-model refactor) removed the per-prefab
//! opening connectivity machinery: the v2 schema authors no per-prefab openings and the
//! migrated content carries none, so inter-fragment connectivity is now by-construction via
//! the 1-cell `default_floor` seam every placement reserves — never an authored, validated
//! per-prefab opening.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

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
