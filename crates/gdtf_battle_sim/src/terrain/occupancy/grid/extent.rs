//! Sparse/dense extent iteration over the grid — the authored/occupied key walk,
//! the authored storey range, the dense all-cells walk, and the index-to-key
//! inverse mapping.

use super::{
    storage::{OccupancyGrid, SlotIndex},
    types::{GRID_HEIGHT, GRID_WIDTH, SLOT_COUNT},
};
use crate::{
    metric::{Cell, CellLevel, Level},
    occupancy::TerrainKind,
};

impl OccupancyGrid {
    /// Iterate the grid's **authored / occupied** `(cell, level)` keys — every slot
    /// whose static terrain is non-[`Open`](TerrainKind::Open) (a wall or cover entry)
    /// OR that holds an occupant — and nothing else.
    ///
    /// The sparse authored/occupied content of the grid (it walks the flat slot buffer
    /// ONCE, never the dense `60 × 60 × 8` extent). The squad-FOV union derives its
    /// **authored level range** ([`authored_level_range`](OccupancyGrid::authored_level_range))
    /// from this set — the storeys the map authors, which bound the union's dense disc
    /// scan (GTW-347). The iterator is read-only and borrows the grid for its lifetime.
    pub fn authored_or_occupied_cells(&self) -> impl Iterator<Item = CellLevel> + '_ {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.terrain != TerrainKind::Open || slot.occupant.is_some())
            .map(|(index, _)| Self::cell_level_of_index(SlotIndex::new(index)))
    }

    /// The inclusive `(min, max)` storey range of the grid's **authored / occupied**
    /// content — the lowest and highest [`Level`] any non-[`Open`](TerrainKind::Open)
    /// terrain or occupant slot sits on — or [`None`] when the grid holds no authored or
    /// occupied content at all (an entirely empty grid).
    ///
    /// This is the **authored level range** the squad-FOV union
    /// ([`union_fov`](crate::visibility::union_fov), GTW-347) sweeps its dense Chebyshev
    /// disc across (unioned with each observer's own storey), so the disc scan spans only
    /// the storeys the map actually authors — a single-level map collapses to `(L, L)`
    /// rather than blind-scanning a fixed `0..`[`MAX_LEVELS`](crate::metric::MAX_LEVELS). Derived from the same
    /// [`authored_or_occupied_cells`](OccupancyGrid::authored_or_occupied_cells) content
    /// (the grid extents), so the two stay in lockstep. A read-only fold over the sparse
    /// authored/occupied keys — it never touches empty air.
    #[must_use]
    pub fn authored_level_range(&self) -> Option<(Level, Level)> {
        self.authored_or_occupied_cells().fold(None, |range, key| {
            // The storey via the canonical `CellLevel::level` accessor (GTW-565).
            let level = key.level();
            Some(match range {
                None => (level, level),
                Some((lo, hi)) => (lo.min(level), hi.max(level)),
            })
        })
    }

    /// Every in-bounds `(cell, level)` of the grid's fixed extent — the full
    /// `GRID_WIDTH × GRID_HEIGHT × MAX_LEVELS` (60 × 60 × 8) cell set, in flat-buffer
    /// (`x + y·WIDTH + level·WIDTH·HEIGHT`) order.
    ///
    /// Walks the buffer's index space `0..SLOT_COUNT` (NOT the slots' contents), so it
    /// yields EVERY structural cell regardless of terrain/occupant — the grid's extent is
    /// fixed once built. This is what
    /// [`SquadVisibility::omniscient`](crate::visibility::SquadVisibility::omniscient)
    /// (GTW-70) materialises into the AI's "every cell visible+explored" move fog. A
    /// read-only iterator that borrows nothing of the grid's contents (the mapping is a
    /// pure index→key function), so the dense `28_800`-cell walk is allocation-free until
    /// the caller collects it.
    pub fn all_cells(&self) -> impl Iterator<Item = CellLevel> {
        (0..SLOT_COUNT).map(|index| Self::cell_level_of_index(SlotIndex::new(index)))
    }

    /// The `(cell, level)` key the flat-buffer slot at `index` represents — the inverse
    /// of [`slot_index`](OccupancyGrid::slot_index) for an in-bounds index.
    ///
    /// `index = x + y·WIDTH + level·WIDTH·HEIGHT`, so the components recover by
    /// successive division/modulo. Used only by
    /// [`authored_or_occupied_cells`](OccupancyGrid::authored_or_occupied_cells) over
    /// the buffer's own indices, which are in `0..SLOT_COUNT` by construction.
    fn cell_level_of_index(index: SlotIndex) -> CellLevel {
        let level = *index / (GRID_WIDTH * GRID_HEIGHT);
        let plane = *index % (GRID_WIDTH * GRID_HEIGHT);
        let y = plane / GRID_WIDTH;
        let x = plane % GRID_WIDTH;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            reason = "x/y are 0..60 and level is 0..MAX_LEVELS (8) by the index's own \
                      construction, so these conversions cannot truncate, wrap, or sign-flip"
        )]
        let key = (x as i32, y as i32, level as u8);
        CellLevel::new(Cell::new(key.0, key.1), Level::new(key.2))
    }
}
