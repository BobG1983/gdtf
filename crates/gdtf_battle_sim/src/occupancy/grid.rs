//! The coarse 3D occupancy grid [`OccupancyGrid`] — the model's `(cell, level)`
//! collision/query surface plus its append-only [`DestroyedCover`] exclusion set.

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::{Deref, Entity, Resource},
};

use crate::{
    cover::HeightBand,
    metric::{Cell, CellLevel, Level, MAX_LEVELS},
    occupancy::{OccupancyInput, TerrainKind},
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
}

impl Default for OccupancyGrid {
    /// An empty grid: every slot [`OccupancySlot::default`] ([`TerrainKind::Open`],
    /// no occupant), no destroyed cover, and no published occupant bands.
    fn default() -> Self {
        Self {
            slots:           vec![OccupancySlot::default(); SLOT_COUNT].into_boxed_slice(),
            destroyed_cover: DestroyedCover::new(),
            occupant_bands:  HashMap::default(),
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
    #[must_use]
    pub fn build_from_occupancy_input(input: &OccupancyInput) -> Self {
        let mut grid = Self::new();
        for placement in &input.terrain {
            grid.set_terrain(placement.at, placement.terrain);
        }
        for placement in &input.occupants {
            // Pour the occupant AND its silhouette band together (GTW-304): the march
            // only strikes a ganger when BOTH are present at a cell, so a placed
            // occupant must carry its band from the first frame.
            grid.set_occupant(placement.at, Some(placement.occupant));
            grid.set_occupant_band(placement.at, Some(placement.band));
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
