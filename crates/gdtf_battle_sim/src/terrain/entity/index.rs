//! The per-battle **terrain cell → entity** lookup index — [`TerrainIndexKey`] (the
//! typed key enum) and [`TerrainIndex`] (the battle-lifetime [`Resource`] map).
//! GTW-395.
//!
//! A cover entity and a slab entity can share the same [`CellLevel`] (e.g. a slab at
//! ground level 0 under a scatter prop); keying the map by `CellLevel` alone would cause
//! one to silently overwrite the other. [`TerrainIndexKey`] is a typed enum
//! (`Cover(CellLevel)` / `Slab(CellLevel)`) so each terrain kind occupies a distinct
//! map key even at the same `(cell, level)`.

use bevy::{
    platform::collections::HashMap,
    prelude::{Entity, Resource},
};

use crate::metric::CellLevel;

/// The typed key for the [`TerrainIndex`] map — discriminates between a cover entity
/// (wall or scatter prop) and a slab entity at the same `(cell, level)`.
///
/// A named domain enum (no-bare-types: a `CellLevel` alone cannot express "is this a
/// cover or slab key?"). The two variants are:
/// - `Cover(CellLevel)` — the cover entity (wall or scatter prop) at this
///   `(cell, level)`.
/// - `Slab(CellLevel)` — the slab entity at this `(cell, level)`.
///
/// This ensures both entities coexist in the map without collision when a cover piece
/// and a slab share a cell (the typed-key no-collision contract, Test 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainIndexKey {
    /// The cover entity (wall or scatter prop) at the given `(cell, level)`.
    Cover(CellLevel),
    /// The slab entity at the given `(cell, level)`.
    Slab(CellLevel),
}

impl TerrainIndexKey {
    /// The `(cell, level)` this key addresses, regardless of kind.
    #[must_use]
    pub const fn cell_level(self) -> CellLevel {
        match self {
            Self::Cover(cl) | Self::Slab(cl) => cl,
        }
    }
}

/// The battle-lifetime **cell → entity** lookup for terrain pieces — a
/// [`HashMap`]`<`[`TerrainIndexKey`]`, `[`Entity`]`>` inserted by `setup_battle` and
/// removed by `teardown_battle_on_request`. GTW-395.
///
/// A Bevy [`Resource`] (one per battle, not one per terrain entity); the sim and
/// presenter look up terrain entities by their typed `(kind, cell)` key rather than
/// scanning the entire world. Distinct from the [`CoverLedger`](crate::cover::CoverLedger)
/// and [`SlabLedger`](crate::slab::SlabLedger) — those own live HP; this is a **pure
/// index** from `(kind, cell)` to the static-stats entity handle.
///
/// The bridge pattern (C3 / Test 6): a system reads `TerrainIndex` to get the entity,
/// reads the entity's [`CoverHp`](crate::cover::CoverHp) (max) off the component, and
/// reads live HP from the `CoverLedger` by the same `CellLevel` key — all three work
/// together, each authoritative for its slice.
#[derive(Resource, Debug, Clone, Default)]
pub struct TerrainIndex {
    /// The one `(kind, cell) → entity` map for every terrain piece spawned this battle.
    entries: HashMap<TerrainIndexKey, Entity>,
}

impl TerrainIndex {
    /// Build the index from a pre-collected iterator of `(key, entity)` pairs.
    ///
    /// Called once by `setup_battle` after all terrain entities are spawned; the pairs
    /// are accumulated inline in the cover + slab spawn loops.
    #[must_use]
    pub fn new(pairs: impl IntoIterator<Item = (TerrainIndexKey, Entity)>) -> Self {
        Self {
            entries: pairs.into_iter().collect(),
        }
    }

    /// Look up the terrain entity for `key` — `None` if no entity was spawned for
    /// that `(kind, cell)`.
    ///
    /// The primary lookup direction: given a typed `(kind, cell)` key, return the
    /// entity that carries the static stats for that terrain piece.
    #[must_use]
    pub fn get(&self, key: &TerrainIndexKey) -> Option<Entity> {
        self.entries.get(key).copied()
    }

    /// The number of entries in the index — the count of terrain entities indexed
    /// this battle (the Test 7 no-collision assertion iterates this).
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the index is empty (no terrain entities were spawned).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
