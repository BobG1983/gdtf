//! The grid's value-type leaves: the [`GRID_WIDTH`] / [`GRID_HEIGHT`] extent
//! constants, the per-slot [`OccupancySlot`] record, and the append-only
//! [`DestroyedCover`] exclusion set.

use bevy::{
    platform::collections::HashSet,
    prelude::{Deref, Entity},
};

use crate::{
    metric::{CellLevel, MAX_LEVELS},
    occupancy::TerrainKind,
};

/// The coarse grid's width in cells on the x ground axis — 60.
///
/// Introduced here (not in E1.1) from `docs/combat/battle-space.md`'s "60×60×8
/// coarse grid" — the hard-maximum arena footprint (`docs/combat/combat.md`:
/// "60×60 is the hard max"). A `usize` because it is the x extent of the flat slot
/// buffer's index space, not a domain value carried in a field.
pub const GRID_WIDTH: usize = 60;

/// The coarse grid's height in cells on the y ground axis — 60.
///
/// Introduced here (not in E1.1) from `docs/combat/battle-space.md`'s "60×60×8
/// coarse grid". A `usize` for the same buffer-extent reason as [`GRID_WIDTH`].
pub const GRID_HEIGHT: usize = 60;

/// One `(cell, level)` slot of the coarse occupancy grid — its static terrain plus
/// the entity (if any) standing in it.
///
/// The slot carries the two facts the collision/query surface reads: the
/// [`TerrainKind`] static-terrain marker, and the **occupant marker**
/// [`occupant`](OccupancySlot::occupant) — an `Option<`[`Entity`]`>` that is a Bevy
/// `Entity` handle, **NEVER a numeric id** (the GTW-10 / GTW-12 architectural
/// constraint). `None` means the slot has no occupant; `Some(e)` means entity `e`
/// stands there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OccupancySlot {
    /// The static-terrain marker for this slot (wall / cover presence vs open).
    pub terrain:  TerrainKind,
    /// The entity occupying this slot, or `None` — a Bevy [`Entity`] handle, never a
    /// numeric id (GTW-10 / GTW-12).
    pub occupant: Option<Entity>,
}

/// The total number of slots in the coarse grid — `GRID_WIDTH × GRID_HEIGHT ×
/// MAX_LEVELS` (60 × 60 × 8). Used to size the flat slot buffer.
pub(super) const SLOT_COUNT: usize = GRID_WIDTH * GRID_HEIGHT * (MAX_LEVELS as usize);

/// The occupancy grid's **append-only** set of `(cell, level)` cells whose cover has
/// been destroyed — the cells excluded from the blocking query (C4).
///
/// A named newtype over the `(cell, level)` set (no-bare-types: the destroyed-cover
/// set is a domain value, not a bare `HashSet`). It is **append-only by
/// construction**: the only mutator is [`mark`](DestroyedCover::mark) (an insert) —
/// there is deliberately **no** remove and **no** mutable [`Deref`], so a destroyed
/// cover cell can never resurrect. It [`Deref`]s read-only to the underlying set for
/// `contains` / `iter` / `len`. This is the occupancy grid's OWN exclusion set,
/// distinct from the GTW-154 [`crate::cover::CoverLedger`]'s HP-depletion flag
/// (syncing the two is GTW-157).
#[derive(Deref, Debug, Clone, Default, PartialEq, Eq)]
pub struct DestroyedCover(HashSet<CellLevel>);

impl DestroyedCover {
    /// Build an empty destroyed-cover set (no cover destroyed yet).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark `cell_level` destroyed — the **only** mutator, an append (insert). There
    /// is no remove, which is what keeps the set append-only: a marked cell stays
    /// excluded for the rest of the battle. Re-marking is a harmless no-op (set
    /// semantics).
    pub fn mark(&mut self, cell_level: CellLevel) {
        self.0.insert(cell_level);
    }
}

/// Whether a `(cell, level)` **blocks** collision / LOS / cover — the
/// destruction-aware answer [`OccupancyGrid::is_blocked`](crate::occupancy::OccupancyGrid::is_blocked)
/// returns, and the static kind-blocking answer
/// [`TerrainKind::blocks`](crate::occupancy::TerrainKind::blocks) reports.
///
/// A named newtype over `bool` (no-bare-types: a cell's blocking-ness is a domain fact,
/// not a bare boolean — a wall/cover cell reads `Blocked(true)`, open air `Blocked(false)`).
/// `true` iff the cell's static terrain blocks AND the cell is not in the destroyed-cover
/// set (a smashed cover cell reads `false`). Distinct from [`PathBlocked`] (the tag-derived
/// PATHFINDING surface) — this is the kind-based collision / vision answer. Private inner +
/// derived [`Deref`] (house style).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Blocked(bool);

impl Blocked {
    /// Build a collision/vision blocking answer from its boolean state.
    #[must_use]
    pub const fn new(blocked: bool) -> Self {
        Self(blocked)
    }
}

/// Whether a `(cell, level)` blocks the **PATH** — the destruction-aware answer
/// [`OccupancyGrid::is_path_blocked`](crate::occupancy::OccupancyGrid::is_path_blocked)
/// returns, and the derivation
/// [`derives_path_blocking`](crate::terrain::def::derives_path_blocking) /
/// [`sim_kind_blocks_path`](crate::terrain::def::sim_kind_blocks_path) reports for a def / kind.
///
/// A named newtype over `bool` (no-bare-types: pathing impassability is a domain fact, not a
/// bare boolean). `true` iff the tag-derived [`PathBlocking`](crate::occupancy::PathBlocking)
/// marker is present AND the cell is not in the destroyed-cover set. Distinct from [`Blocked`]
/// (the kind-based collision / vision surface): the pathfinder reads THIS, vision reads
/// [`Blocked`]. Private inner + derived [`Deref`] (house style).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PathBlocked(bool);

impl PathBlocked {
    /// Build a path-blocking answer from its boolean state.
    #[must_use]
    pub const fn new(blocked: bool) -> Self {
        Self(blocked)
    }
}

/// Whether a `(cell, level)` / terrain kind **occludes vision** at all — the bare presence
/// answer [`OccupancyGrid::occludes_vision`](crate::occupancy::OccupancyGrid::occludes_vision)
/// returns for a height-tested sightline, and
/// [`sim_kind_occludes_vision`](crate::terrain::def::sim_kind_occludes_vision) reports for a kind.
///
/// A named newtype over `bool` (no-bare-types: vision-occlusion presence is a domain fact,
/// not a bare boolean). The height-AWARE occluder BAND is the separately-typed
/// [`BlocksVision`](crate::terrain::entity::BlocksVision) / the
/// [`VisionBlocking`](crate::occupancy::VisionBlocking) surface; this bool is the yes/no
/// presence answer only. Private inner + derived [`Deref`] (house style).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OccludesVision(bool);

impl OccludesVision {
    /// Build a vision-occlusion presence answer from its boolean state.
    #[must_use]
    pub const fn new(occludes: bool) -> Self {
        Self(occludes)
    }
}

/// Whether a `(cell, level)`'s cover has been **destroyed** — the membership answer
/// [`OccupancyGrid::is_cover_destroyed`](crate::occupancy::OccupancyGrid::is_cover_destroyed)
/// returns (the cell is in the append-only [`DestroyedCover`] set).
///
/// A named newtype over `bool` (no-bare-types: cover-destruction is a domain fact, not a bare
/// boolean). `true` when the cell has been marked destroyed via
/// [`OccupancyGrid::mark_cover_destroyed`](crate::occupancy::OccupancyGrid::mark_cover_destroyed),
/// gating the destroyed-cover exclusion in the blocking / vision queries. Private inner +
/// derived [`Deref`] (house style).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverDestroyed(bool);

impl CoverDestroyed {
    /// Build a cover-destroyed answer from its boolean state.
    #[must_use]
    pub const fn new(destroyed: bool) -> Self {
        Self(destroyed)
    }
}
