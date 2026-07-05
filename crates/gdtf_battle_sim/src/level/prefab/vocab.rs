//! The **shared prefab schema vocabulary** — [`SpawnRole`] and [`PrefabName`], the
//! role/name leaves the [`PrefabRegistry`](super::PrefabRegistry) keys prefabs by.

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
/// (`role: Fill`), and [`Hash`]/[`Eq`] so the [`PrefabKey`](super::PrefabKey) can key on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum SpawnRole {
    /// A fragment that hosts the human player's deployment zone.
    Player,
    /// A fragment that hosts the enemy deployment zone.
    Enemy,
    /// A generic connective / interior fragment (neither deployment zone).
    Fill,
}

/// A prefab's **name** — the stable identifier the [`PrefabRegistry`](super::PrefabRegistry) records and the
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
