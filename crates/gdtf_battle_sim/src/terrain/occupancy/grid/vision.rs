//! The GTW-502 height-aware vision-occlusion query surface — the tag-derived
//! occluder map the LoS/FoV march reads.

use super::{storage::OccupancyGrid, types::OccludesVision};
use crate::{cover::HeightBand, metric::CellLevel};

impl OccupancyGrid {
    /// The **vision-occluder band** at `cell_level` — `Some(band)` iff its tag-derived
    /// [`VisionBlocking`](crate::occupancy::VisionBlocking) occluder is present AND the cell is NOT in the destroyed-cover set,
    /// else `None` (GTW-502 C3).
    ///
    /// This is the height-aware VISION query the GTW-502 occluder introduces — it reads the
    /// tag-derived [`VisionBlocking`](crate::occupancy::VisionBlocking) surface PROJECTED from the
    /// [`BlocksVision`](crate::terrain::entity::BlocksVision) components, NOT the kind-based
    /// [`terrain`](OccupancyGrid::terrain) marker or the GTW-501 path surface. The band is the
    /// one a sightline must fly STRICTLY HIGHER than to clear the occluder (the
    /// [`round_clears_occupant`](crate::clearance::round_clears_occupant) gate, the same the
    /// occupant + cover clauses use).
    ///
    /// The destroyed-cover exclusion MIRRORS [`is_path_blocked`](OccupancyGrid::is_path_blocked)
    /// / [`is_blocked`](OccupancyGrid::is_blocked): a `(cell, level)` in
    /// [`destroyed_cover`](OccupancyGrid::destroyed_cover) reads `None` even if it carries an
    /// occluder — a destroyed wall opens the sightline, exactly as it stops occluding via its
    /// `CoverLedger` entry, so the new surface never re-blocks a sightline the cover destruction
    /// already opened (zero-regression, no double-count). An out-of-range or unmarked
    /// `cell_level` reads `None` (graceful — no panic).
    #[must_use]
    pub fn vision_occluder_at(&self, cell_level: &CellLevel) -> Option<HeightBand> {
        if *self.is_cover_destroyed(cell_level) {
            return None;
        }
        self.vision_blocking.get(cell_level).copied()
    }

    /// Whether a sightline at `test_band` is OCCLUDED at `cell_level` — `true` iff the cell
    /// carries a vision occluder (via [`vision_occluder_at`](OccupancyGrid::vision_occluder_at))
    /// AND the sightline does not fly strictly higher than the occluder's band (GTW-502 C5).
    ///
    /// The height-aware gate the LoS/FoV march reads: it REUSES
    /// [`round_clears_occupant`](crate::clearance::round_clears_occupant) — equal-or-lower
    /// [`Clearance::Impacts`](crate::clearance::Clearance::Impacts) (occluded), strictly-higher
    /// clears (not occluded) — so a LOW occluder blocks a LOW sightline but a HIGH one sails
    /// over it, identically to how cover bands gate the march. A destroyed-cover or unmarked
    /// cell reads `false` (graceful — no panic).
    #[must_use]
    pub fn occludes_vision(&self, cell_level: &CellLevel, test_band: HeightBand) -> OccludesVision {
        OccludesVision::new(self.vision_occluder_at(cell_level).is_some_and(|band| {
            crate::clearance::round_clears_occupant(test_band, band)
                == crate::clearance::Clearance::Impacts
        }))
    }

    /// Mark `cell_level` as **vision-occluding** at `band` — insert it into the tag-derived
    /// [`VisionBlocking`](crate::occupancy::VisionBlocking) surface read by
    /// [`vision_occluder_at`](OccupancyGrid::vision_occluder_at) (GTW-502 C4).
    ///
    /// The write [`project_vision_blocking`](crate::occupancy::project_vision_blocking) makes
    /// for an `Added`/`Changed`<`BlocksVision`> component (the setup-spawn insert, a later
    /// runtime add, AND a band re-tune). Re-inserting OVERWRITES the band (a retuned occluder
    /// updates the cell in place). Edits the surface IN PLACE — it never rebuilds the grid.
    pub fn set_vision_blocking(&mut self, cell_level: CellLevel, band: HeightBand) {
        self.vision_blocking.insert(cell_level, band);
    }

    /// Clear `cell_level`'s **vision-occlusion** — remove it from the tag-derived
    /// [`VisionBlocking`](crate::occupancy::VisionBlocking) surface read by
    /// [`vision_occluder_at`](OccupancyGrid::vision_occluder_at) (GTW-502 C4).
    ///
    /// The write [`project_vision_blocking`](crate::occupancy::project_vision_blocking) makes
    /// for a `RemovedComponents<BlocksVision>` component — re-opening the sightline the next
    /// LoS/FoV query reads. Clearing an unmarked cell is a harmless no-op. Edits IN PLACE.
    pub fn clear_vision_blocking(&mut self, cell_level: CellLevel) {
        self.vision_blocking.remove(&cell_level);
    }
}
