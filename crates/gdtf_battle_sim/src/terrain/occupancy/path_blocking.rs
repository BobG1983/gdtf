//! The **tag-derived path-blocking surface** (GTW-501, child 482a) — the
//! [`PathBlocking`] cell set the [`OccupancyGrid`] holds, plus the
//! [`project_path_blocking`] system that keeps it in sync with the
//! [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) markers.
//!
//! ## Why a separate path surface (GTW-501 D1)
//!
//! Before this child a single kind-based
//! [`OccupancyGrid::is_blocked`](super::OccupancyGrid::is_blocked) was shared by BOTH
//! vision (the LOS/cover march) and pathfinding. GTW-501 SPLITS path-blocking out: the
//! pathfinder now reads this tag-derived surface
//! ([`OccupancyGrid::is_path_blocked`](super::OccupancyGrid::is_path_blocked)) while
//! vision keeps reading the kind-based `is_blocked` UNCHANGED (vision's own occluder is
//! GTW-502). So this surface is path-ONLY.
//!
//! ## Source of truth vs. projected snapshot (GTW-501 C2 / C3)
//!
//! The [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) markers on terrain
//! entities are the SOURCE OF TRUTH (derived from the def at spawn, GTW-501 C1). This
//! [`PathBlocking`] set is the PROJECTED snapshot the pathfinder reads — kept in sync by
//! [`project_path_blocking`] via `Added<BlocksPathfinding>` /
//! `RemovedComponents<BlocksPathfinding>` change detection, so adding the marker re-blocks
//! a cell and removing it re-opens one with no full rebuild.

use bevy::{
    ecs::{entity::Entity, system::SystemParam},
    platform::collections::{HashMap, HashSet},
    prelude::{Added, Deref, Local, Query, RemovedComponents, ResMut},
};

use super::OccupancyGrid;
use crate::{
    metric::CellLevel,
    terrain::entity::{BlocksPathfinding, TerrainCell},
};

/// The occupancy grid's **tag-derived path-blocking** `(cell, level)` set — the projected
/// snapshot of the [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) markers
/// that the pathfinder reads (GTW-501 C2).
///
/// A named newtype over the `(cell, level)` set (no-bare-types: the path-blocking surface
/// is a domain value, not a bare `HashSet`). Private inner + derived read-only [`Deref`]
/// to the set for `contains` / `len` / `iter`; mutation goes through the named
/// [`insert`](PathBlocking::insert) / [`remove`](PathBlocking::remove) methods (so the
/// invariant — only the projection writes it — is concentrated here). Distinct from the
/// append-only [`DestroyedCover`](super::DestroyedCover) set: this one is two-way (a marker
/// can be added OR removed at runtime, GTW-501 C3), and it is keyed by the marker's cell
/// rather than a destruction record.
#[derive(Deref, Debug, Clone, Default, PartialEq, Eq)]
pub struct PathBlocking(HashSet<CellLevel>);

impl PathBlocking {
    /// Build an empty path-blocking set (no cell blocks the path yet).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark `cell_level` path-blocking — the insert the projection makes for an
    /// `Added<BlocksPathfinding>` marker. Re-inserting is a harmless no-op (set semantics).
    pub fn insert(&mut self, cell_level: CellLevel) {
        self.0.insert(cell_level);
    }

    /// Clear `cell_level`'s path-blocking flag — the removal the projection makes for a
    /// `RemovedComponents<BlocksPathfinding>` marker. Removing an absent cell is a harmless
    /// no-op (set semantics).
    pub fn remove(&mut self, cell_level: &CellLevel) {
        self.0.remove(cell_level);
    }
}

/// The terrain-entity reads the [`project_path_blocking`] system needs, bundled as one
/// [`SystemParam`] so the system signature reads cleanly (GTW-501 C3).
///
/// Two disjoint queries plus the [`RemovedComponents`] reader (a special `SystemParam` that
/// must be drained every run, `bevy-traps.md` — it is NOT a `Query`):
///
/// - `added` — every terrain entity that JUST gained a
///   [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) marker this tick
///   (`Added` fires on insert, including the setup-spawn insert), with its [`Entity`] +
///   [`TerrainCell`] so the projection knows which cell to block and can record the
///   entity→cell mapping for a later removal.
/// - `removed` — the `Entity`s whose `BlocksPathfinding` marker was removed this tick. A
///   `RemovedComponents` yields ONLY the `Entity` — and on a DESPAWN the entity is already
///   gone, so its [`TerrainCell`] can no longer be queried; that is why the system keeps the
///   entity→cell map in a `Local` (see [`project_path_blocking`]).
#[derive(SystemParam)]
pub struct PathBlockingChanges<'w, 's> {
    /// Terrain entities that gained the marker this tick (incl. the setup-spawn insert),
    /// paired with the entity so the projection can record the entity→cell mapping.
    added:   Query<'w, 's, (Entity, &'static TerrainCell), Added<BlocksPathfinding>>,
    /// The entities whose `BlocksPathfinding` marker was removed this tick (component-remove
    /// OR despawn). Only the `Entity` is available — the cell is recovered from the `Local`.
    removed: RemovedComponents<'w, 's, BlocksPathfinding>,
}

/// Project the [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) markers
/// into the [`OccupancyGrid`]'s [`PathBlocking`] surface, keeping it in sync via
/// change detection (GTW-501 C3).
///
/// Two halves, both reconciled against the entity→cell map the system keeps in a `Local`:
///
/// - **Added** — every `Added<BlocksPathfinding>` (which fires for the setup-spawn insert
///   AND a later runtime add) marks its cell path-blocking on the grid AND records the
///   entity→cell mapping. This is how the surface is populated at setup AND re-blocked when a
///   marker is added at runtime (C3 / C6d).
/// - **Removed** — every `RemovedComponents<BlocksPathfinding>` clears the cell it last
///   blocked, re-opening the path (C3 / C6d). `RemovedComponents` is a special `SystemParam`
///   that MUST be drained each run (`bevy-traps.md`) — the loop reads it every tick. It yields
///   only the `Entity`, and on a DESPAWN the entity is already gone (so its `TerrainCell` can
///   no longer be queried); the `tracked` `Local` map holds the cell so removal works for BOTH
///   a component-remove and a despawn.
///
/// `tracked` is a system-private [`Local`] entity→cell map: a marker's cell is recorded on
/// add and forgotten on removal. It is the projection's OWN reverse index, NOT a domain
/// resource — system-private scratch state, so it stays a plain `HashMap` (`no-bare-types`
/// rule 4: framework plumbing the system owns).
///
/// ORDERING (C3): chained `.before` the pathfinding consumers (`dispatch_move` /
/// `advance_walk` / `enemy_ai_turn` order `.after(project_path_blocking)`), so a tick's
/// marker change is visible to the same tick's path query — the projection writes the grid
/// directly, and a same-tick `Res<OccupancyGrid>` read sees it (`bevy-traps.md` #3). It
/// mutates the grid IN PLACE; it NEVER rebuilds it.
///
/// `bevy-traps.md` #7: a normal system over `Query` / `ResMut` / `Local` — no `&mut World`.
pub fn project_path_blocking(
    mut grid: ResMut<OccupancyGrid>,
    mut changes: PathBlockingChanges,
    mut tracked: Local<HashMap<Entity, CellLevel>>,
) {
    // Removed first: a cell that lost AND regained the marker in the same tick (e.g. a
    // toggling test) ends BLOCKED — the add below wins, matching the marker's final state.
    for entity in changes.removed.read() {
        if let Some(cell) = tracked.remove(&entity) {
            grid.clear_path_blocking(cell);
        }
    }
    for (entity, cell) in &changes.added {
        grid.set_path_blocking(**cell);
        tracked.insert(entity, **cell);
    }
}
