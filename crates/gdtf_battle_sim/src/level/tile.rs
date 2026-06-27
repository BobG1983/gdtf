//! The per-tile catalog entry — a named, atlas-indexed, stat-carrying tile the editor
//! palette lists and the procgen assembly places (GTW-409).
//!
//! Every field is a named newtype with a private inner (no-bare-types): a
//! [`TileDisplayName`] (the palette label), a [`TileAtlasIndex`] (the OPAQUE atlas
//! index — just data, the sim never renders it), and a [`CatalogTileKind`] payload that
//! REUSES the existing terrain-piece stat newtypes so the editor shows the SAME
//! gameplay stats the combat path reads. The catalog is render-free: it carries the
//! atlas INDEX, it does not resolve or draw it (the presenter does).

use bevy::prelude::Deref;
use serde::Deserialize;

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    tuning::MoveCost,
};

/// A catalog tile's **display name** — the human label the editor palette shows beside
/// the tile (GTW-409).
///
/// A presentation-label newtype over [`String`] (no-bare-types rule 1: a label string
/// is a domain value). Private inner + derived [`Deref`]; `#[serde(transparent)]` parses
/// a bare RON string. Distinct from [`TileKey`](super::TileKey) (the catalog-internal
/// key) and from [`TerrainName`](crate::terrain::piece::TerrainName) (a terrain-file
/// stem) — a name a designer READS, not a key the code resolves on.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct TileDisplayName(String);

impl TileDisplayName {
    /// Build a tile display name from its label string.
    #[must_use]
    pub const fn new(label: String) -> Self {
        Self(label)
    }
}

/// A catalog tile's **atlas index** — the OPAQUE index into the presenter's tile atlas
/// sheet (GTW-409).
///
/// Just DATA the sim stores render-free (the [`TerrainGraphicKey`](crate::terrain::piece::TerrainGraphicKey)
/// precedent — an opaque presentation hook): the sim never resolves it to a pixel and
/// the sim NEVER depends on the presenter. The presenter reads it to pick the sprite
/// for a tile (a 0-based row-major index into `assets/sprites/alt_tileset_terrain.png`,
/// matching the existing `tile_roles.spritedef.ron` index vocabulary).
///
/// A presentation-hook newtype over [`usize`] (no-bare-types rule 1: an atlas index is
/// a domain value). Private inner + derived [`Deref`]; `#[serde(transparent)]` parses a
/// bare RON integer.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct TileAtlasIndex(usize);

impl TileAtlasIndex {
    /// Build an atlas index from its row-major sheet index.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

/// The **kind-specific gameplay stats** of a catalog tile — which terrain kind the tile
/// places and the stats the editor shows for it (GTW-409).
///
/// Mirrors [`TerrainKindSpec`](crate::terrain::piece::TerrainKindSpec)'s five-kind shape
/// (FLOOR / WALL / COVER / SCATTER / SLAB) so the catalog tile carries the SAME gameplay
/// stats the combat path reads — REUSING the same stat newtypes ([`MoveCost`] /
/// [`CoverHp`] / [`SlabHp`] / [`ArmorProtection`] / [`ArmorHardness`] / [`HeightBand`]),
/// no-bare-types throughout. These are the "stats shown bottom-right in the editor" (C4);
/// they are tuning DATA authored in the `.theme.ron`, not pinned by tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum CatalogTileKind {
    /// Walkable ground — carries a terrain move cost (the editor shows the step cost).
    Floor {
        /// The TU cost to step onto this floor (REUSE [`MoveCost`]).
        move_cost: MoveCost,
    },
    /// Solid blocking geometry — a destructible structural piece (HP / armor / band).
    Wall(StructuralStats),
    /// Chest-high cover prop — a destructible structural piece a round may clear.
    Cover(StructuralStats),
    /// Loose scatter / debris prop — a destructible structural piece.
    Scatter(StructuralStats),
    /// Floor/roof slab — a destructible structural piece spanning a z-boundary; NO
    /// height band (the [`SlabPieceSpec`](crate::terrain::piece::SlabPieceSpec)
    /// precedent — a slab spans the whole boundary, so there is no band to clear).
    Slab {
        /// The full structural HP the slab seeds to (REUSE [`SlabHp`]).
        max_hp:           SlabHp,
        /// The damage-reduction stat (REUSE [`ArmorProtection`]).
        armor_protection: ArmorProtection,
        /// The penetration the slab shrugs off (REUSE [`ArmorHardness`]).
        armor_hardness:   ArmorHardness,
    },
}

/// The wall / cover / scatter gameplay stats shown in the editor — HP, armor, and
/// clearance band (GTW-409).
///
/// Shared by the three structural [`CatalogTileKind`] variants, mirroring
/// [`StructuralSpec`](crate::terrain::piece::StructuralSpec) (the wall-or-prop "one
/// shape for both" precedent). Every field REUSES the existing combat stat newtype, so
/// the palette shows exactly the stats the march reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct StructuralStats {
    /// The full structural HP the piece seeds to (REUSE [`CoverHp`]).
    pub max_hp:           CoverHp,
    /// The damage-reduction stat — same armor model as a ganger (REUSE [`ArmorProtection`]).
    pub armor_protection: ArmorProtection,
    /// The penetration this piece shrugs off (REUSE [`ArmorHardness`]).
    pub armor_hardness:   ArmorHardness,
    /// The clearance band this piece occupies — LOW / MID / HIGH (REUSE [`HeightBand`]).
    pub height_band:      HeightBand,
}

/// One **catalog tile** — a named, atlas-indexed, stat-carrying entry in a theme's tile
/// catalog (GTW-409).
///
/// The unit the editor palette lists (C4: display name + atlas index + gameplay stats)
/// and the procgen assembly places. Every field is a named newtype with a private
/// inner: a [`TileDisplayName`], a [`TileAtlasIndex`] (opaque, render-free), and a
/// [`CatalogTileKind`] carrying the gameplay stats. **Not `Copy`** — the
/// [`TileDisplayName`] owns a `String`; it is `Clone` so the registry can hold tiles
/// by value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct CatalogTile {
    /// The human label the editor palette shows.
    pub display_name: TileDisplayName,
    /// The OPAQUE atlas index the presenter resolves to a sprite (the sim never renders it).
    pub atlas_index:  TileAtlasIndex,
    /// The kind-specific gameplay stats the editor shows and the procgen assembly places.
    pub kind:         CatalogTileKind,
}
