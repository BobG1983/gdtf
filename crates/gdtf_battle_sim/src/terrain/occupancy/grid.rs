//! The coarse 3D occupancy grid [`OccupancyGrid`] — the model's `(cell, level)`
//! collision/query surface plus its append-only [`DestroyedCover`] exclusion set
//! and the stair-cell eye-offset map ([`StairEyeOffset`] / [`OccupancyGrid::is_stair_cell`]).

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::{Deref, Entity, Resource},
};

use crate::{
    cover::HeightBand,
    metric::{Cell, CellLevel, Level, MAX_LEVELS},
    occupancy::{OccupancyInput, TerrainKind},
};

/// The **eye z-lift** applied to an observer standing on an authored stair tile
/// (GTW-390).
///
/// A stair tile sits at the junction between storeys: the observer's body spans
/// two levels, so their eye is higher than a flat-floor observer at the same
/// `(cell, level)`. This offset is ADDED to `f32::from(level) +
/// muzzle_height(stance)` in the `eye_anchor` probe function (GTW-390) to reflect
/// that extra height — **stance-gated at the use site** (Prone → 0.0, any upright
/// posture → +0.5).
///
/// Non-stair cells are simply absent from the
/// [`OccupancyGrid`]'s stair-cell set, so
/// [`OccupancyGrid::stair_eye_offset_at`] returns `StairEyeOffset(0.0)` by default
/// (i.e. `Default`). Private inner + derived [`Deref`] (no-bare-types house style).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Default)]
pub struct StairEyeOffset(f32);

impl StairEyeOffset {
    /// Build a stair eye-offset from the raw level-fraction `offset`.
    ///
    /// Pass `0.0` for a non-stair cell or a Prone observer (no lift); pass `0.5`
    /// for a Standing or Crouching observer on an authored stair tile.
    #[must_use]
    pub const fn new(offset: f32) -> Self {
        Self(offset)
    }
}

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
const SLOT_COUNT: usize = GRID_WIDTH * GRID_HEIGHT * (MAX_LEVELS as usize);

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

/// The coarse 3D occupancy grid — the model's `(cell, level)` collision/query
/// surface that movement (GTW-12) and the LOS/cover queries read (the coarse
/// occupancy the authoritative model owns; ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`).
///
/// A Bevy [`Resource`] (one grid per battle). It holds [`SLOT_COUNT`]
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
    slots:           Box<[OccupancySlot]>,
    /// The append-only set of `(cell, level)` cells whose cover has been destroyed —
    /// excluded from [`is_blocked`](OccupancyGrid::is_blocked); see
    /// [`mark_cover_destroyed`](OccupancyGrid::mark_cover_destroyed). The occupancy
    /// grid's OWN set, distinct from [`crate::cover::CoverLedger`] (C4).
    destroyed_cover: DestroyedCover,
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
    occupant_bands:  HashMap<CellLevel, HeightBand>,
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
    stair_cells:     HashSet<CellLevel>,
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

    /// Mark `cell_level` as an **authored stair tile** (GTW-390) — insert it into the
    /// [`stair_cells`](OccupancyGrid::is_stair_cell) set so the LOS probe can lift
    /// the eye z for an observer standing on a stair endpoint.
    ///
    /// Called at battle setup for every endpoint of every
    /// [`Stair`](crate::vertical::LinkKind::Stair) link in the situation's
    /// vertical-link list ([`Ladder`](crate::vertical::LinkKind::Ladder) endpoints
    /// are excluded — ladders do not share the half-level stair eye-lift). Re-marking
    /// the same cell is a harmless no-op (set semantics).
    pub fn mark_stair_cell(&mut self, cell_level: CellLevel) {
        self.stair_cells.insert(cell_level);
    }

    /// Whether `cell_level` is an authored stair tile — a read-only membership test.
    ///
    /// Returns `true` when the cell was registered via
    /// [`mark_stair_cell`](OccupancyGrid::mark_stair_cell) at battle setup (it is a
    /// stair endpoint). A cell absent from the set — including every cell on a grid
    /// built with `OccupancyGrid::new()` — returns `false` (non-stair = no eye-lift).
    #[must_use]
    pub fn is_stair_cell(&self, cell_level: &CellLevel) -> bool {
        self.stair_cells.contains(cell_level)
    }

    /// The raw [`StairEyeOffset`] for `cell_level` — `StairEyeOffset(0.5)` when the
    /// cell is an authored stair tile, `StairEyeOffset(0.0)` otherwise (non-stair or
    /// out-of-range).
    ///
    /// This is the **un-gated** offset: the caller (`eye_anchor` in
    /// [`crate::los`]) applies the **stance gate**
    /// (`Prone → 0.0`, any upright posture → read this value) before adding it to the
    /// eye z. Returning `0.5` for every non-Prone stair observer and `0.0` for every
    /// other combination keeps the gate logic concentrated at the `eye_anchor` call
    /// site.
    #[must_use]
    pub fn stair_eye_offset_at(&self, cell_level: &CellLevel) -> StairEyeOffset {
        if self.stair_cells.contains(cell_level) {
            StairEyeOffset::new(0.5)
        } else {
            StairEyeOffset::new(0.0)
        }
    }

    /// Register a ganger's **stair presence** atomically — write the lower cell and,
    /// when safe, the upper cell — returning the upper [`CellLevel`] written so the
    /// caller can record it in [`PrevSlot`](crate::occupancy_sync::PrevSlot) for
    /// verbatim teardown (GTW-391).
    ///
    /// Writes the lower cell unconditionally: `set_occupant(lower, Some(entity))` +
    /// `set_occupant_band(lower, Some(lower_band))`.
    ///
    /// Then attempts the upper cell `(cell, level+1)`:
    ///
    /// * Returns `None` (lower-only) when there is no upper cell (top storey —
    ///   `level == MAX_LEVELS - 1`).
    /// * Returns `None` (lower-only) when the upper cell is already occupied by a
    ///   **different** entity — the **occupancy guard** (Blocker 3): the single-occupant
    ///   slot must not be stomped. A ganger at `(cell, level+1)` as its own lower cell
    ///   would lose its presence if we overwrote its slot here.
    /// * Writes `set_occupant(upper, Some(entity))` + `set_occupant_band(upper,
    ///   Some(HeightBand::Low))` and returns `Some(upper)` when the upper cell is
    ///   unoccupied, OR already owned by the SAME entity (idempotent re-register: a
    ///   ganger re-posing upright on the same stair just refreshes the band).
    ///
    /// The caller ([`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers) /
    /// [`build_from_occupancy_input`](OccupancyGrid::build_from_occupancy_input)) stores
    /// the returned `Option<CellLevel>` in `PrevSlot` so every teardown path can replay
    /// the exact written set without re-derivation.
    pub fn register_stair_presence(
        &mut self,
        lower: CellLevel,
        entity: bevy::prelude::Entity,
        lower_band: HeightBand,
    ) -> Option<CellLevel> {
        // Always write the lower cell.
        self.set_occupant(lower, Some(entity));
        self.set_occupant_band(lower, Some(lower_band));

        // Compute the cell directly above — None at the top storey.
        let upper = Self::upper_cell(&lower)?;

        // OCCUPANCY GUARD (Blocker 3): never stomp a cell owned by a different entity.
        match self.occupant(&upper) {
            None => {
                // Upper cell is free — claim it.
                self.set_occupant(upper, Some(entity));
                self.set_occupant_band(upper, Some(HeightBand::Low));
                Some(upper)
            }
            Some(other) if other == entity => {
                // Idempotent re-register: same entity already owns the upper cell.
                // Refresh the band (e.g. upright re-pose after a stance change).
                self.set_occupant_band(upper, Some(HeightBand::Low));
                Some(upper)
            }
            Some(_) => {
                // Upper cell owned by a different entity — lower-only (no stomp).
                None
            }
        }
    }

    /// Clear `entity`'s upper-cell stair presence at `upper` (occupant + band),
    /// guarded by ownership so it never stomps a slot another entity now owns (GTW-391).
    ///
    /// Only clears when `occupant(&upper) == Some(entity)` — if the cell was taken
    /// over by a different entity in the meantime, this is a harmless no-op. Used by
    /// [`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers) and
    /// [`sync_dead_gangers`](crate::occupancy_sync::sync_dead_gangers) to replay the
    /// upper-cell teardown recorded in `PrevSlot::upper`.
    pub fn clear_stair_upper(&mut self, upper: CellLevel, entity: bevy::prelude::Entity) {
        if self.occupant(&upper) == Some(entity) {
            self.set_occupant(upper, None);
            self.set_occupant_band(upper, None);
        }
    }

    /// The `(cell, level+1)` cell directly above `lower`, or `None` when `lower` is at
    /// the top storey (`level == MAX_LEVELS - 1`) — the upper-presence target for a
    /// stair occupant (GTW-391).
    ///
    /// Private: only [`register_stair_presence`](OccupancyGrid::register_stair_presence)
    /// calls this. The arithmetic is `level + 1` checked against `MAX_LEVELS`.
    fn upper_cell(lower: &CellLevel) -> Option<CellLevel> {
        // `lower.z` is a storey index stored as `i32` in `CellLevel`'s `IVec3`; cast to
        // `u8` for the checked add (the storey index is always in `0..MAX_LEVELS`).
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "lower.z is a storey index in 0..MAX_LEVELS (8) by construction, \
                      so the i32 -> u8 narrowing cannot truncate or sign-flip"
        )]
        let z = lower.z as u8;
        let next = z.checked_add(1)?;
        if next >= MAX_LEVELS {
            return None;
        }
        Some(CellLevel::new(
            Cell::new(lower.x, lower.y),
            Level::new(next),
        ))
    }

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
    pub fn is_cover_destroyed(&self, cell_level: &CellLevel) -> bool {
        self.destroyed_cover.contains(cell_level)
    }

    /// Whether `cell_level` **blocks** (collision / LOS / cover) — `true` if its
    /// static terrain blocks AND the cell is NOT in the destroyed-cover set (C6).
    ///
    /// A destroyed cover cell does **NOT** block: a `(cell, level)` in
    /// [`destroyed_cover`](OccupancyGrid::destroyed_cover) returns `false` even if its
    /// terrain marker is [`TerrainKind::Cover`] (or [`TerrainKind::Wall`]) — the
    /// smashed piece no longer obstructs (smashed walls/props do not resurrect;
    /// `docs/combat/combat.md`: cover "can be shot and destroyed"). An out-of-range `cell_level` reads [`TerrainKind::Open`] and so
    /// returns `false` (graceful — no panic).
    #[must_use]
    pub fn is_blocked(&self, cell_level: &CellLevel) -> bool {
        if self.is_cover_destroyed(cell_level) {
            return false;
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
            .map(|(index, _)| Self::cell_level_of_index(index))
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
    /// rather than blind-scanning a fixed `0..`[`MAX_LEVELS`]. Derived from the same
    /// [`authored_or_occupied_cells`](OccupancyGrid::authored_or_occupied_cells) content
    /// (the grid extents), so the two stay in lockstep. A read-only fold over the sparse
    /// authored/occupied keys — it never touches empty air.
    #[must_use]
    pub fn authored_level_range(&self) -> Option<(Level, Level)> {
        self.authored_or_occupied_cells().fold(None, |range, key| {
            // `CellLevel` derefs `IVec3`; its `z` is the storey index, 0..MAX_LEVELS by
            // construction (the slot buffer never holds an out-of-range level).
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "key.z is a storey index in 0..MAX_LEVELS (8) by the slot buffer's \
                          own construction, so the i32 -> u8 narrowing cannot truncate or \
                          sign-flip"
            )]
            let level = Level::new(key.z as u8);
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
        (0..SLOT_COUNT).map(Self::cell_level_of_index)
    }

    /// The `(cell, level)` key the flat-buffer slot at `index` represents — the inverse
    /// of [`slot_index`](OccupancyGrid::slot_index) for an in-bounds index.
    ///
    /// `index = x + y·WIDTH + level·WIDTH·HEIGHT`, so the components recover by
    /// successive division/modulo. Used only by
    /// [`authored_or_occupied_cells`](OccupancyGrid::authored_or_occupied_cells) over
    /// the buffer's own indices, which are in `0..SLOT_COUNT` by construction.
    fn cell_level_of_index(index: usize) -> CellLevel {
        let level = index / (GRID_WIDTH * GRID_HEIGHT);
        let plane = index % (GRID_WIDTH * GRID_HEIGHT);
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
