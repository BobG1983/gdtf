//! One **placed terrain piece** in a [`PrefabSpecV2`](super::PrefabSpecV2) — the
//! [`TerrainPlacementEntry`], a `(piece, at)` pair (GTW-486).

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use crate::{metric::CellLevel, terrain::def::TerrainUuid};

/// One **placed terrain piece** in a v2 prefab — *which* terrain definition goes *where*.
///
/// The single entry shape the v2 [`placements`](super::PrefabSpecV2::placements) list is
/// built from: a stable [`TerrainUuid`] reference to a
/// [`TerrainDef`](crate::terrain::def::TerrainDef) (the [`piece`](TerrainPlacementEntry::piece))
/// at a `(cell, level)` footprint position (the [`at`](TerrainPlacementEntry::at)). It
/// REPLACES the legacy schema's four split lists (walls / scatter / slabs / floors): the
/// per-piece behaviour (wall vs scatter vs slab vs floor, HP, move cost) now lives in the
/// referenced [`TerrainDef`](crate::terrain::def::TerrainDef), so a placement carries only
/// the reference and the location.
///
/// Both leaf fields are NAMED newtypes (no-bare-types rule 1: a terrain reference and a
/// grid position are domain values, never a bare `Uuid` / `IVec3`) — the existing
/// [`TerrainUuid`] (GTW-484) and [`CellLevel`] (the sim's `(cell, level)` key) reused
/// verbatim, not redeclared. Derives [`Deserialize`] / [`Serialize`] so an entry parses
/// from and round-trips through its authoring shape, and [`TypePath`] so it can ride a
/// reflected payload the way the sibling spec types do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TypePath)]
pub struct TerrainPlacementEntry {
    /// The stable [`TerrainUuid`] reference to the [`TerrainDef`](crate::terrain::def::TerrainDef)
    /// this placement spawns.
    pub piece: TerrainUuid,
    /// The `(cell, level)` footprint position the piece is placed at.
    pub at:    CellLevel,
}

impl TerrainPlacementEntry {
    /// Build a placement of a terrain `piece` at a `(cell, level)` footprint position.
    #[must_use]
    pub const fn new(piece: TerrainUuid, at: CellLevel) -> Self {
        Self { piece, at }
    }
}
