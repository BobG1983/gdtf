//! The [`OccupancyGrid`] struct itself, its constructors, and the slot-level
//! read/write choke points (terrain / occupant / occupant-band accessors over the
//! flat slot buffer).

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::{Entity, Resource},
};

use super::types::{DestroyedCover, GRID_HEIGHT, GRID_WIDTH, OccupancySlot, SLOT_COUNT};
use crate::{
    cover::HeightBand,
    metric::{CellLevel, MAX_LEVELS},
    occupancy::{OccupancyInput, PathBlocking, TerrainKind, VisionBlocking},
};

/// The coarse 3D occupancy grid — the model's `(cell, level)` collision/query
/// surface that movement (GTW-12) and the LOS/cover queries read (the coarse
/// occupancy the authoritative model owns; ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`).
///
/// A Bevy [`Resource`] (one grid per battle). It holds `SLOT_COUNT`
/// ([`GRID_WIDTH`] × [`GRID_HEIGHT`] × [`MAX_LEVELS`] = 60 × 60 × 8) flat
/// [`OccupancySlot`]s — a slot per `(cell, level)` — addressed by
/// `x + y·WIDTH + level·WIDTH·HEIGHT`. The grid is built from the situation's
/// static terrain plus live ganger state, so
/// [`build_from_occupancy_input`](OccupancyGrid::build_from_occupancy_input) is the
/// canonical constructor.
///
/// [`destroyed_cover`](OccupancyGrid::destroyed_cover) is the grid's own
/// **append-only** exclusion set: once a cell is in it, it can never resurrect on a
/// rebuild, and it is excluded from [`is_blocked`](OccupancyGrid::is_blocked). It is
/// **distinct** from the GTW-154 [`crate::cover::CoverLedger`]'s HP-depletion
/// [`crate::cover::Destroyed`] flag (syncing the two is GTW-157).
///
/// [`stair_cells`](OccupancyGrid::is_stair_cell) is the authored stair-tile set
/// (GTW-390): cells populated from the situation's
/// [`VerticalLink`](crate::vertical::VerticalLink) list at battle setup (stair
/// endpoints only; ladders are excluded). The LOS probe reads
/// [`stair_eye_offset_at`](OccupancyGrid::stair_eye_offset_at) per observer to lift
/// the eye z by half a level on an authored stair tile.
#[derive(Resource, Debug, Clone)]
pub struct OccupancyGrid {
    /// The flat `(cell, level)` slot buffer — `SLOT_COUNT` slots addressed by
    /// [`slot_index`](OccupancyGrid::slot_index). A `Box<[_]>` (fixed length once
    /// built) rather than a growable `Vec`, so the grid's extent is structural.
    pub(super) slots:           Box<[OccupancySlot]>,
    /// The append-only set of `(cell, level)` cells whose cover has been destroyed —
    /// excluded from [`is_blocked`](OccupancyGrid::is_blocked); see
    /// [`mark_cover_destroyed`](OccupancyGrid::mark_cover_destroyed). The occupancy
    /// grid's OWN set, distinct from [`crate::cover::CoverLedger`] (C4).
    pub(super) destroyed_cover: DestroyedCover,
    /// The per-`(cell, level)` **silhouette band** of the occupant standing there
    /// (a ganger's stance band — prone Low / kneel Mid / stand High), or absent when
    /// the slot has no occupant or no band has been published yet.
    ///
    /// The march bands the round vs the occupant by reading this directly off the
    /// grid (`docs/combat/resolution.md` §2: "the round's continuous z vs the
    /// occupant's band-top"): a ganger's stance lives on its components, so the
    /// change-driven maintenance publishes the derived band here when it marks the
    /// occupant, and the band-free march reads it back. A lazily-populated side map
    /// (like [`destroyed_cover`](OccupancyGrid::destroyed_cover)) so it is fully
    /// backward compatible with a grid built without occupant bands (the band reads
    /// `None`).
    pub(super) occupant_bands:  HashMap<CellLevel, HeightBand>,
    /// The authored **stair-tile** cell set (GTW-390) — every `(cell, level)` that is
    /// an endpoint of a [`Stair`](crate::vertical::LinkKind::Stair) link in the
    /// situation's vertical-link graph.
    ///
    /// Populated at battle setup from the situation's
    /// [`VerticalLink`](crate::vertical::VerticalLink) list (stair endpoints only;
    /// [`Ladder`](crate::vertical::LinkKind::Ladder) endpoints are excluded). The LOS
    /// probe calls [`stair_eye_offset_at`](OccupancyGrid::stair_eye_offset_at) per
    /// observer: a cell present here returns `StairEyeOffset(0.5)` (stance-gated to
    /// `0.0` for Prone at the call site); an absent cell returns `StairEyeOffset(0.0)`.
    pub(super) stair_cells:     HashSet<CellLevel>,
    /// The **tag-derived path-blocking** surface (GTW-501) — the projected snapshot of
    /// the [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) markers that
    /// the PATHFINDER reads via [`is_path_blocked`](OccupancyGrid::is_path_blocked).
    ///
    /// SEPARATE from the kind-based [`terrain`](OccupancySlot::terrain) /
    /// [`is_blocked`](OccupancyGrid::is_blocked) surface VISION still reads (the GTW-501 D1
    /// split): pathfinding decides impassability from THIS set (tags, not kind), vision is
    /// untouched. Kept in sync by
    /// [`project_path_blocking`](crate::occupancy::project_path_blocking) via
    /// `Added`/`RemovedComponents` change detection. A lazily-populated side map (like
    /// [`destroyed_cover`](OccupancyGrid::destroyed_cover)), so a grid built without the
    /// projection (e.g. `OccupancyGrid::new()` in a unit test) simply reads every cell as
    /// non-path-blocking.
    pub(super) path_blocking:   PathBlocking,
    /// The **tag-derived vision-blocking** surface (GTW-502) — the projected snapshot of the
    /// [`BlocksVision`](crate::terrain::entity::BlocksVision) components that the LoS/FoV
    /// march ([`impact_at`](crate::march)) reads via
    /// [`vision_occluder_at`](OccupancyGrid::vision_occluder_at) /
    /// [`occludes_vision`](OccupancyGrid::occludes_vision).
    ///
    /// A `(cell, level) → `[`HeightBand`] MAP (not a set like `path_blocking`): vision
    /// occlusion is HEIGHT-AWARE, so the surface records WHICH band each cell occludes (a
    /// sightline must fly strictly higher than the band to clear it). SEPARATE from BOTH the
    /// kind-based [`is_blocked`](OccupancyGrid::is_blocked) (a movement-collision query the
    /// vision march does NOT read) and the GTW-501 path-blocking surface (the two are
    /// independent — GTW-502 C7). It is ADDITIVE to the existing occupant + cover occlusion
    /// `impact_at` already performs: an intact `Wall`/`Cover` still occludes via its
    /// [`CoverLedger`](crate::cover::CoverLedger) entry (the cover clause fires first), so this
    /// surface re-derives a `Cover`/`Emplacement`'s ledger band exactly and, for every shipped
    /// `Wall` (all `High`), an equal band (idempotent, no double-count) and ADDS occlusion only
    /// for an explicitly-`BlocksVision`-tagged `Slab` (the gap-closer). Kept in
    /// sync by [`project_vision_blocking`](crate::occupancy::project_vision_blocking) via
    /// `Added`/`Changed`/`RemovedComponents` change detection. A lazily-populated side map
    /// (like [`destroyed_cover`](OccupancyGrid::destroyed_cover)), so a grid built without the
    /// projection (e.g. `OccupancyGrid::new()` in a unit test) reads every cell as
    /// non-occluding.
    pub(super) vision_blocking: VisionBlocking,
}

impl Default for OccupancyGrid {
    /// An empty grid: every slot [`OccupancySlot::default`] ([`TerrainKind::Open`],
    /// no occupant), no destroyed cover, no published occupant bands, and no stair
    /// cells.
    fn default() -> Self {
        Self {
            slots:           vec![OccupancySlot::default(); SLOT_COUNT].into_boxed_slice(),
            destroyed_cover: DestroyedCover::new(),
            occupant_bands:  HashMap::default(),
            stair_cells:     HashSet::default(),
            path_blocking:   PathBlocking::new(),
            vision_blocking: VisionBlocking::new(),
        }
    }
}

impl OccupancyGrid {
    /// Build an empty occupancy grid — every slot [`TerrainKind::Open`] with no
    /// occupant, and no destroyed cover.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a fresh grid from an [`OccupancyInput`], pouring its terrain and
    /// occupant placements into the slots — the grid's constructor, built from the
    /// situation's static terrain plus live ganger state.
    ///
    /// Starts from an empty grid (all [`TerrainKind::Open`], no occupants), then sets
    /// each [`TerrainPlacement`](crate::occupancy::TerrainPlacement)'s slot terrain
    /// and each [`OccupantPlacement`](crate::occupancy::OccupantPlacement)'s slot
    /// occupant **and** its silhouette band (set together — GTW-304: a placed
    /// occupant carries its band so the march can strike it from the first frame).
    /// Placements whose `(cell, level)` falls **outside** the grid extent
    /// are skipped (graceful — no panic, the bounds-check rule). The new grid's
    /// [`destroyed_cover`](OccupancyGrid::destroyed_cover) set starts empty; carrying
    /// destroyed cover across a rebuild is the caller's append (E1.7 / GTW-157).
    ///
    /// **GTW-391 stair cells.** `stair_cells` is the set of authored stair-tile
    /// `(cell, level)` positions (from [`mark_stair_cell`](OccupancyGrid::mark_stair_cell)
    /// — built by the caller before this call so the stair set is populated before
    /// occupant pour). For each placed occupant whose cell is in `stair_cells` AND
    /// whose band is not [`HeightBand::Low`] (non-prone — Low is the prone silhouette),
    /// [`register_stair_presence`](OccupancyGrid::register_stair_presence) is called
    /// instead of the plain lower-cell write: it writes the upper `(cell, level+1)`
    /// presence synchronously, so the upper cell exists from **frame 0** (no first-tick
    /// gap, Blocker 2 resolved). A band of `Low` at placement = prone = lower-cell only.
    #[must_use]
    pub fn build_from_occupancy_input(
        input: &OccupancyInput,
        stair_cells: &bevy::platform::collections::HashSet<CellLevel>,
    ) -> Self {
        let mut grid = Self::new();
        // First, mark every authored stair cell (so is_stair_cell is populated before
        // the occupant pour — the caller has already computed stair_cells from the
        // vertical-link list and passes it in; we mark here to keep the grid consistent).
        for &cell in stair_cells {
            grid.mark_stair_cell(cell);
        }
        for placement in &input.terrain {
            grid.set_terrain(placement.at, placement.terrain);
        }
        for placement in &input.occupants {
            // Pour the occupant AND its silhouette band together (GTW-304): the march
            // only strikes a ganger when BOTH are present at a cell, so a placed
            // occupant must carry its band from the first frame.
            //
            // GTW-391: if the placement is on a stair cell AND not prone (Low band =
            // prone silhouette), use register_stair_presence for dual-cell occupancy.
            // The returned upper cell is intentionally dropped here (initial-placement
            // PrevSlot will be written by the first sync_moved_gangers Changed<Position>
            // tick — the placement feeds into the ECS occupancy sync, not directly into
            // PrevSlot at setup time).
            if stair_cells.contains(&placement.at) && placement.band != HeightBand::Low {
                let _ =
                    grid.register_stair_presence(placement.at, placement.occupant, placement.band);
            } else {
                grid.set_occupant(placement.at, Some(placement.occupant));
                grid.set_occupant_band(placement.at, Some(placement.band));
            }
        }
        grid
    }

    /// The flat-buffer index for `key`, or `None` if `key` is **outside** the grid
    /// extent (`0..GRID_WIDTH` × `0..GRID_HEIGHT` × `0..MAX_LEVELS`).
    ///
    /// The single bounds-check choke point: negative coordinates and any axis at or
    /// past its extent return `None`, so every public accessor degrades gracefully
    /// (no panic, no out-of-bounds index) on an out-of-range coordinate.
    fn slot_index(key: &CellLevel) -> Option<usize> {
        let x = usize::try_from(key.x).ok()?;
        let y = usize::try_from(key.y).ok()?;
        let level = usize::try_from(key.z).ok()?;
        if x >= GRID_WIDTH || y >= GRID_HEIGHT || level >= MAX_LEVELS as usize {
            return None;
        }
        Some(x + y * GRID_WIDTH + level * GRID_WIDTH * GRID_HEIGHT)
    }

    /// The slot at `key`, or `None` if `key` is outside the grid — a read-only peek
    /// that never mutates the grid and never panics on an out-of-range coordinate.
    #[must_use]
    pub fn slot(&self, key: &CellLevel) -> Option<&OccupancySlot> {
        Self::slot_index(key).and_then(|i| self.slots.get(i))
    }

    /// Set the static-terrain marker at `key`. An out-of-range `key` is a graceful
    /// no-op (the bounds-check rule — no panic).
    ///
    /// Used by
    /// [`build_from_occupancy_input`](OccupancyGrid::build_from_occupancy_input) to
    /// pour the situation's authored walls / cover into the grid.
    pub fn set_terrain(&mut self, key: CellLevel, terrain: TerrainKind) {
        if let Some(slot) = Self::slot_index(&key).and_then(|i| self.slots.get_mut(i)) {
            slot.terrain = terrain;
        }
    }

    /// Set the occupant at `key` (a Bevy [`Entity`] handle, never a numeric id), or
    /// clear it with `None`. An out-of-range `key` is a graceful no-op.
    ///
    /// Used by
    /// [`build_from_occupancy_input`](OccupancyGrid::build_from_occupancy_input) to
    /// pour the situation's live occupants into the grid.
    pub fn set_occupant(&mut self, key: CellLevel, occupant: Option<Entity>) {
        if let Some(slot) = Self::slot_index(&key).and_then(|i| self.slots.get_mut(i)) {
            slot.occupant = occupant;
        }
    }

    /// The terrain marker at `key`, defaulting to [`TerrainKind::Open`] for an
    /// out-of-range `key` — a read-only peek that never panics.
    #[must_use]
    pub fn terrain(&self, key: &CellLevel) -> TerrainKind {
        self.slot(key).map_or(TerrainKind::Open, |s| s.terrain)
    }

    /// The occupant at `key`, or `None` if the slot is empty **or** `key` is
    /// out-of-range — a read-only peek that never panics. The returned handle is a
    /// Bevy [`Entity`], never a numeric id.
    #[must_use]
    pub fn occupant(&self, key: &CellLevel) -> Option<Entity> {
        self.slot(key).and_then(|s| s.occupant)
    }

    /// The **silhouette band** of the occupant at `key` (its stance band), or `None`
    /// if no band has been published there — the band the march compares the round
    /// against when it crosses a ganger-occupied cell (`docs/combat/resolution.md`
    /// §2). A read-only peek; an out-of-range `key` simply has no entry, so it reads
    /// `None` (graceful — no panic).
    #[must_use]
    pub fn occupant_band(&self, key: &CellLevel) -> Option<HeightBand> {
        self.occupant_bands.get(key).copied()
    }

    /// Publish (or clear with `None`) the occupant's silhouette band at `key` — the
    /// write the change-driven maintenance makes when it marks/moves an occupant, so
    /// the band-free [`crate::march::march_vector`] can read the occupant's band back
    /// off the grid (`docs/combat/resolution.md` §2). Keyed by `(cell, level)` like
    /// the destroyed-cover set, so it never touches the flat slot buffer and stays
    /// backward compatible with a grid built without occupant bands.
    pub fn set_occupant_band(&mut self, key: CellLevel, band: Option<HeightBand>) {
        match band {
            Some(band) => {
                self.occupant_bands.insert(key, band);
            }
            None => {
                self.occupant_bands.remove(&key);
            }
        }
    }
}
