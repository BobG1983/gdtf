//! The destroyed-cover exclusion plus the movement-collision
//! ([`OccupancyGrid::is_blocked`]) and GTW-501 path-blocking
//! ([`OccupancyGrid::is_path_blocked`]) query surfaces.

use super::{
    storage::OccupancyGrid,
    types::{Blocked, CoverDestroyed, DestroyedCover, PathBlocked},
};
use crate::metric::CellLevel;

impl OccupancyGrid {
    /// Mark the cover at `cell_level` **destroyed**, inserting it into the
    /// append-only [`destroyed_cover`](OccupancyGrid::destroyed_cover) set — a cell
    /// in that set can never resurrect (C4): a destroyed-cover set keeps smashed
    /// walls/props from resurrecting.
    ///
    /// **Append-only:** this only ever inserts — there is no API on the grid that
    /// removes a cell from the set, so a destroyed cover cell stays excluded for the
    /// rest of the battle. Calling it again on the same cell is a harmless no-op (set
    /// semantics). This is the occupancy grid's OWN exclusion set, distinct from the
    /// [`crate::cover::CoverLedger`]'s HP flag (syncing them is GTW-157). The
    /// `cell_level` is recorded regardless of whether it is in-range — the set is a
    /// pure exclusion record keyed by `(cell, level)`.
    pub fn mark_cover_destroyed(&mut self, cell_level: CellLevel) {
        self.destroyed_cover.mark(cell_level);
    }

    /// Whether `cell_level` is in the append-only destroyed-cover set — a read-only
    /// peek.
    #[must_use]
    pub fn is_cover_destroyed(&self, cell_level: &CellLevel) -> CoverDestroyed {
        CoverDestroyed::new(self.destroyed_cover.contains(cell_level))
    }

    /// Whether `cell_level` **blocks** (collision / LOS / cover) — `true` if its
    /// static terrain blocks AND the cell is NOT in the destroyed-cover set (C6).
    ///
    /// A destroyed cover cell does **NOT** block: a `(cell, level)` in
    /// [`destroyed_cover`](OccupancyGrid::destroyed_cover) returns `false` even if its
    /// terrain marker is [`TerrainKind::Cover`](crate::occupancy::TerrainKind::Cover) (or [`TerrainKind::Wall`](crate::occupancy::TerrainKind::Wall)) — the
    /// smashed piece no longer obstructs (smashed walls/props do not resurrect;
    /// `docs/combat/combat.md`: cover "can be shot and destroyed"). An out-of-range `cell_level` reads [`TerrainKind::Open`](crate::occupancy::TerrainKind::Open) and so
    /// returns `false` (graceful — no panic).
    #[must_use]
    pub fn is_blocked(&self, cell_level: &CellLevel) -> Blocked {
        if *self.is_cover_destroyed(cell_level) {
            return Blocked::new(false);
        }
        self.terrain(cell_level).blocks()
    }

    /// A read-only handle on the append-only [`DestroyedCover`] set — for callers
    /// that need to iterate or count the excluded cells (e.g. carrying them across a
    /// rebuild, E1.7). Growing the set goes through
    /// [`mark_cover_destroyed`](OccupancyGrid::mark_cover_destroyed), keeping it
    /// append-only.
    #[must_use]
    pub const fn destroyed_cover(&self) -> &DestroyedCover {
        &self.destroyed_cover
    }

    /// Whether `cell_level` blocks the **PATH** (GTW-501) — `true` iff its
    /// tag-derived [`PathBlocking`](crate::occupancy::PathBlocking) marker is present AND the cell is NOT in the
    /// destroyed-cover set.
    ///
    /// This is the PATHFINDING-only blocking query the GTW-501 D1 split introduces — it
    /// reads the tag-derived [`PathBlocking`](crate::occupancy::PathBlocking) surface PROJECTED from the
    /// [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) markers, NOT the
    /// kind-based [`terrain`](OccupancyGrid::terrain) marker. So the pathfinder decides
    /// impassability from the tags, never from [`TerrainKind`](crate::occupancy::TerrainKind) (the epic criterion). VISION
    /// keeps reading [`is_blocked`](OccupancyGrid::is_blocked) unchanged.
    ///
    /// The destroyed-cover exclusion MIRRORS [`is_blocked`](OccupancyGrid::is_blocked): a
    /// `(cell, level)` in [`destroyed_cover`](OccupancyGrid::destroyed_cover) reads `false`
    /// even if it carries a path-blocking marker — a smashed wall/prop no longer obstructs a
    /// path, so a destroyed cover cell re-opens the route exactly as it does for vision
    /// (the GTW-501 C5 zero-path-regression guarantee — no new destruction wiring needed).
    /// An out-of-range or unmarked `cell_level` reads `false` (graceful — no panic).
    #[must_use]
    pub fn is_path_blocked(&self, cell_level: &CellLevel) -> PathBlocked {
        if *self.is_cover_destroyed(cell_level) {
            return PathBlocked::new(false);
        }
        PathBlocked::new(self.path_blocking.contains(cell_level))
    }

    /// Mark `cell_level` as **path-blocking** — insert it into the tag-derived
    /// [`PathBlocking`](crate::occupancy::PathBlocking) surface read by [`is_path_blocked`](OccupancyGrid::is_path_blocked)
    /// (GTW-501 C3).
    ///
    /// The write [`project_path_blocking`](crate::occupancy::project_path_blocking) makes
    /// for an `Added<BlocksPathfinding>` marker (the setup-spawn insert AND a later runtime
    /// add). Re-marking is a harmless no-op (set semantics). Edits the surface IN PLACE — it
    /// never rebuilds the grid.
    pub fn set_path_blocking(&mut self, cell_level: CellLevel) {
        self.path_blocking.insert(cell_level);
    }

    /// Clear `cell_level`'s **path-blocking** flag — remove it from the tag-derived
    /// [`PathBlocking`](crate::occupancy::PathBlocking) surface read by [`is_path_blocked`](OccupancyGrid::is_path_blocked)
    /// (GTW-501 C3).
    ///
    /// The write [`project_path_blocking`](crate::occupancy::project_path_blocking) makes
    /// for a `RemovedComponents<BlocksPathfinding>` marker — re-opening the path the next
    /// query reads. Clearing an unmarked cell is a harmless no-op. Edits IN PLACE.
    pub fn clear_path_blocking(&mut self, cell_level: CellLevel) {
        self.path_blocking.remove(&cell_level);
    }
}
