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
