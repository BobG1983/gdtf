//! Coarse 3D occupancy grid: the model's `(cell, level)` collision/query surface —
//! the static terrain plus the live occupant of every slot of the 60×60×8 coarse
//! grid (`docs/combat/battle-space.md`: "the 60×60×8 coarse grid";
//! `docs/architecture.md`: "the **coarse occupancy** (a 3D grid, e.g. 60×60×8)").
//!
//! This is the E1.6 occupancy-grid slice. The grid is what the collision/query
//! surface movement (GTW-12) and the LOS/cover queries read: each `(cell, level)`
//! slot carries a **static-terrain marker** ([`TerrainKind`] — wall / cover
//! presence, blocking vs non-blocking) and an **occupant marker**
//! ([`OccupancySlot::occupant`], an `Option<`[`Entity`]`>`). The occupant is ALWAYS
//! a Bevy [`Entity`] handle, **never a numeric id** — the GTW-10 / GTW-12
//! architectural constraint that ids never cross as raw integers into the grid.
//!
//! Per `docs/architecture.md` the occupancy is "rebuilt **fresh per shot** from the
//! situation's static terrain plus live ganger state: always correct by
//! construction, no stale-sync seam", and "a destroyed-cover set keeps smashed
//! walls/props from resurrecting on the rebuild". This module supplies both halves:
//!
//! 1. [`OccupancyGrid::build_from_occupancy_input`] pours an [`OccupancyInput`]'s
//!    terrain and occupant placements into a fresh grid — the grid's constructor.
//!    The full situation→entities setup orchestration is GTW-158 (E1.8), which owns
//!    the canonical authored [`crate::situation::Situation`]; the [`OccupancyInput`]
//!    shape here is the **grid-relevant slice only** (terrain placements + occupant
//!    placements carrying [`Entity`] handles) that the setup derives from the
//!    spawned ganger entities and the authored walls/scatter.
//! 2. [`OccupancyGrid::destroyed_cover`] is an **append-only** exclusion set: a cell
//!    marked via [`OccupancyGrid::mark_cover_destroyed`] can never resurrect, and it
//!    is excluded from the blocking query [`OccupancyGrid::is_blocked`] (a destroyed
//!    cover cell does NOT block). This is the occupancy grid's **own** exclusion
//!    set, **distinct** from the GTW-154 [`crate::cover::CoverLedger`]'s HP-depletion
//!    [`crate::cover::Destroyed`] flag — syncing the two is GTW-157 (E1.7), not here.
//!
//! The grid dimensions are STRUCTURAL constants: [`GRID_WIDTH`] / [`GRID_HEIGHT`]
//! (60×60, introduced here from `docs/combat/battle-space.md`) × [`MAX_LEVELS`] (8,
//! from E1.1). Out-of-range coordinates are handled **gracefully** — a query/mark on
//! a cell outside the grid is a no-op / "not blocked", never a panic.

use bevy::{
    platform::collections::HashSet,
    prelude::{Deref, Entity, Resource},
};

use crate::metric::{CellLevel, MAX_LEVELS};

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

/// The static-terrain marker for one `(cell, level)` slot — what kind of fixed
/// terrain occupies it, and therefore whether it **blocks** (collision / LOS /
/// cover).
///
/// A named domain enum (no-bare-types: terrain presence is a domain value, not a
/// bare `Option<bool>`), carrying the two categories `docs/combat/combat.md` names
/// for fixed geometry — walls and cover — alongside open space:
///
/// - [`Open`](TerrainKind::Open): empty space — no fixed terrain, **non-blocking**.
/// - [`Wall`](TerrainKind::Wall): a wall — solid fixed geometry, **blocking**
///   (`docs/combat/combat.md`: walls are part of the coarse geometry a shot/LOS
///   flies through).
/// - [`Cover`](TerrainKind::Cover): a piece of cover (wall-height or a prop) present
///   in this slot — **blocking** while it stands, but excluded once the cell is in
///   the [`destroyed_cover`](OccupancyGrid::destroyed_cover) set
///   (`docs/combat/combat.md`: cover "can be shot and destroyed").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TerrainKind {
    /// Empty space — no fixed terrain here; **non-blocking**.
    #[default]
    Open,
    /// A wall — solid fixed geometry; **blocking**.
    Wall,
    /// A piece of cover present in this slot — **blocking** until the cell is marked
    /// destroyed (then excluded from the blocking query).
    Cover,
}

impl TerrainKind {
    /// Whether this terrain kind blocks **on its own** — `true` for [`Wall`] and
    /// [`Cover`], `false` for [`Open`].
    ///
    /// This is the *static* blocking-ness of the terrain marker alone; it does NOT
    /// account for the destroyed-cover exclusion (a destroyed [`Cover`] cell still
    /// reads `true` here but is excluded by [`OccupancyGrid::is_blocked`]). Callers
    /// wanting the live, destruction-aware answer use [`OccupancyGrid::is_blocked`].
    ///
    /// [`Wall`]: TerrainKind::Wall
    /// [`Cover`]: TerrainKind::Cover
    /// [`Open`]: TerrainKind::Open
    #[must_use]
    pub const fn blocks(self) -> bool {
        matches!(self, Self::Wall | Self::Cover)
    }
}

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

/// A terrain placement in an [`OccupancyInput`] — a `(cell, level)` slot and the
/// [`TerrainKind`] authored there.
///
/// The grid-relevant slice of a situation's static geometry (walls / cover poured
/// into the grid). A named struct rather than a bare `(CellLevel, TerrainKind)`
/// tuple so the grid's input shape is self-describing. The full situation
/// authoring/orchestration is GTW-158.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainPlacement {
    /// The `(cell, level)` this terrain occupies.
    pub at:      CellLevel,
    /// The kind of terrain authored at [`at`](TerrainPlacement::at).
    pub terrain: TerrainKind,
}

impl TerrainPlacement {
    /// Build a terrain placement from its `(cell, level)` and [`TerrainKind`].
    #[must_use]
    pub const fn new(at: CellLevel, terrain: TerrainKind) -> Self {
        Self { at, terrain }
    }
}

/// An occupant placement in an [`OccupancyInput`] — a `(cell, level)` slot and the
/// Bevy [`Entity`] standing there.
///
/// The grid-relevant slice of a situation's live occupants (gangers poured into the
/// grid). Carries an [`Entity`] handle, **never a numeric id** (GTW-10 / GTW-12). A
/// named struct rather than a bare `(CellLevel, Entity)` tuple so the input shape is
/// self-describing. The full situation authoring/orchestration is GTW-158.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OccupantPlacement {
    /// The `(cell, level)` the occupant stands in.
    pub at:       CellLevel,
    /// The entity occupying [`at`](OccupantPlacement::at) — a Bevy [`Entity`], never
    /// a numeric id.
    pub occupant: Entity,
}

impl OccupantPlacement {
    /// Build an occupant placement from its `(cell, level)` and the occupying
    /// [`Entity`].
    #[must_use]
    pub const fn new(at: CellLevel, occupant: Entity) -> Self {
        Self { at, occupant }
    }
}

/// The grid-relevant **construction input** for the coarse occupancy grid — the
/// terrain and occupant placements [`OccupancyGrid::build_from_occupancy_input`]
/// pours into a fresh grid.
///
/// This is the INPUT shape E1.6 defines (renamed from the GTW-156 placeholder
/// `Situation` by GTW-158, which makes the authored [`crate::situation::Situation`]
/// the one canonical situation type): the static-terrain placements
/// ([`TerrainPlacement`]) and the live-occupant placements ([`OccupantPlacement`],
/// each carrying an [`Entity`] handle). It is deliberately the **grid-relevant
/// slice only** — the GTW-158 setup (E1.8) derives an `OccupancyInput` from the
/// canonical situation (terrain from the authored walls / scatter, occupants from
/// the SPAWNED ganger entities) and pours it through here.
#[derive(Debug, Clone, Default)]
pub struct OccupancyInput {
    /// The static-terrain placements (walls / cover) to pour into the grid.
    pub terrain:   Vec<TerrainPlacement>,
    /// The live-occupant placements (entities) to pour into the grid.
    pub occupants: Vec<OccupantPlacement>,
}

impl OccupancyInput {
    /// Build an empty occupancy input (no terrain, no occupants).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
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
/// surface that movement (GTW-12) and the LOS/cover queries read
/// (`docs/architecture.md`'s "coarse occupancy").
///
/// A Bevy [`Resource`] (one grid per battle). It holds [`SLOT_COUNT`]
/// ([`GRID_WIDTH`] × [`GRID_HEIGHT`] × [`MAX_LEVELS`] = 60 × 60 × 8) flat
/// [`OccupancySlot`]s — a slot per `(cell, level)` — addressed by
/// `x + y·WIDTH + level·WIDTH·HEIGHT`. Per `docs/architecture.md` the grid is built
/// "from the situation's static terrain plus live ganger state", so
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
}

impl Default for OccupancyGrid {
    /// An empty grid: every slot [`OccupancySlot::default`] ([`TerrainKind::Open`],
    /// no occupant) and no destroyed cover.
    fn default() -> Self {
        Self {
            slots:           vec![OccupancySlot::default(); SLOT_COUNT].into_boxed_slice(),
            destroyed_cover: DestroyedCover::new(),
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
    /// occupant placements into the slots — the grid's constructor
    /// (`docs/architecture.md`: built "from the situation's static terrain plus live
    /// ganger state").
    ///
    /// Starts from an empty grid (all [`TerrainKind::Open`], no occupants), then sets
    /// each [`TerrainPlacement`]'s slot terrain and each [`OccupantPlacement`]'s slot
    /// occupant. Placements whose `(cell, level)` falls **outside** the grid extent
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
            grid.set_occupant(placement.at, Some(placement.occupant));
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

    /// Mark the cover at `cell_level` **destroyed**, inserting it into the
    /// append-only [`destroyed_cover`](OccupancyGrid::destroyed_cover) set — a cell
    /// in that set can never resurrect (C4; `docs/architecture.md`: "a
    /// destroyed-cover set keeps smashed walls/props from resurrecting on the
    /// rebuild").
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
    /// smashed piece no longer obstructs (`docs/architecture.md`: smashed
    /// walls/props do not resurrect; `docs/combat/combat.md`: cover "can be shot and
    /// destroyed"). An out-of-range `cell_level` reads [`TerrainKind::Open`] and so
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
}

#[cfg(test)]
mod tests {
    use bevy::ecs::world::World;

    use super::*;
    use crate::metric::{Cell, Level};

    fn key(x: i32, y: i32, level: u8) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(level))
    }

    /// The grid extent constants as `i32` cell coordinates — a checked conversion
    /// (`GRID_WIDTH`/`GRID_HEIGHT` are `usize` so a raw `as i32` cast trips
    /// `cast_possible_wrap`). The fallback is unreachable for the 60-cell extents but
    /// keeps the test free of `unwrap`/`expect` (denied in tests too).
    fn extent_i32(extent: usize) -> i32 {
        i32::try_from(extent).unwrap_or(i32::MAX)
    }

    /// C8(a) — `build_from_occupancy_input` from a HAND-BUILT input (entities
    /// spawned in a real Bevy `World`, terrain placed) populates terrain + occupant
    /// slots correctly, asserted CELL-BY-CELL.
    ///
    /// Spawns two real entities, hand-builds an `OccupancyInput` with a wall, a
    /// cover, and those two occupants at distinct `(cell, level)`s, builds the grid,
    /// then walks every authored slot and asserts BOTH its terrain marker and its
    /// occupant handle, plus that an untouched slot is `Open` / empty. The occupant
    /// is the spawned `Entity` handle, never a numeric id (GTW-10 / GTW-12).
    #[test]
    fn build_from_input_populates_terrain_and_occupant_cell_by_cell() {
        // Real entities from a real World (the C8(a) requirement).
        let mut world = World::new();
        let alice = world.spawn_empty().id();
        let bob = world.spawn_empty().id();

        let wall_at = key(1, 2, 0);
        let cover_at = key(3, 4, 1);
        let alice_at = key(5, 6, 0);
        let bob_at = key(7, 8, 2);
        let empty_at = key(10, 10, 0);

        let input = OccupancyInput {
            terrain:   vec![
                TerrainPlacement::new(wall_at, TerrainKind::Wall),
                TerrainPlacement::new(cover_at, TerrainKind::Cover),
            ],
            occupants: vec![
                OccupantPlacement::new(alice_at, alice),
                OccupantPlacement::new(bob_at, bob),
            ],
        };

        let grid = OccupancyGrid::build_from_occupancy_input(&input);

        // Terrain, cell by cell.
        assert_eq!(
            grid.terrain(&wall_at),
            TerrainKind::Wall,
            "the wall slot must carry TerrainKind::Wall",
        );
        assert_eq!(
            grid.terrain(&cover_at),
            TerrainKind::Cover,
            "the cover slot must carry TerrainKind::Cover",
        );
        // Occupants, cell by cell — the exact spawned Entity handles.
        assert_eq!(
            grid.occupant(&alice_at),
            Some(alice),
            "alice's slot must hold alice's Entity handle",
        );
        assert_eq!(
            grid.occupant(&bob_at),
            Some(bob),
            "bob's slot must hold bob's Entity handle",
        );
        // A slot the situation never touched is Open with no occupant.
        assert_eq!(
            grid.terrain(&empty_at),
            TerrainKind::Open,
            "an untouched slot must default to Open",
        );
        assert_eq!(
            grid.occupant(&empty_at),
            None,
            "an untouched slot must have no occupant",
        );
        // A terrain slot carries no occupant and an occupant slot is Open terrain —
        // the two facts are independent per slot.
        assert_eq!(grid.occupant(&wall_at), None);
        assert_eq!(grid.terrain(&alice_at), TerrainKind::Open);
    }

    /// C8(b) — marking a cell destroyed EXCLUDES it from the blocking query.
    ///
    /// A cover cell blocks while it stands; after `mark_cover_destroyed` it must
    /// NOT block (C6), while an unrelated standing cover cell still blocks — proving
    /// the exclusion is per-cell, not global.
    #[test]
    fn destroyed_cover_is_excluded_from_blocking() {
        let input = OccupancyInput {
            terrain:   vec![
                TerrainPlacement::new(key(2, 2, 0), TerrainKind::Cover),
                TerrainPlacement::new(key(9, 9, 0), TerrainKind::Cover),
            ],
            occupants: Vec::new(),
        };
        let mut grid = OccupancyGrid::build_from_occupancy_input(&input);

        let smashed = key(2, 2, 0);
        let intact = key(9, 9, 0);

        // Both cover cells block while standing.
        assert!(grid.is_blocked(&smashed), "standing cover must block");
        assert!(grid.is_blocked(&intact), "standing cover must block");

        // Mark one destroyed — it must no longer block, the other still blocks.
        grid.mark_cover_destroyed(smashed);
        assert!(
            !grid.is_blocked(&smashed),
            "a destroyed cover cell must NOT block (C6)",
        );
        assert!(
            grid.is_blocked(&intact),
            "an unrelated standing cover cell must still block",
        );
        // The destroyed-cover set records the smashed cell.
        assert!(grid.is_cover_destroyed(&smashed));
        assert!(!grid.is_cover_destroyed(&intact));
    }

    /// A wall blocks and Open does not — the static blocking-ness of the terrain
    /// marker (independent of destruction).
    #[test]
    fn wall_blocks_open_does_not() {
        let input = OccupancyInput {
            terrain:   vec![TerrainPlacement::new(key(4, 4, 0), TerrainKind::Wall)],
            occupants: Vec::new(),
        };
        let grid = OccupancyGrid::build_from_occupancy_input(&input);

        assert!(grid.is_blocked(&key(4, 4, 0)), "a wall must block");
        assert!(
            !grid.is_blocked(&key(0, 0, 0)),
            "an Open cell must not block",
        );
        assert!(TerrainKind::Wall.blocks());
        assert!(TerrainKind::Cover.blocks());
        assert!(!TerrainKind::Open.blocks());
    }

    /// The destroyed-cover set is **append-only** and survives a rebuild only by
    /// re-appending: a freshly built grid starts with an empty set, and the only way
    /// to grow it is `mark_cover_destroyed` (there is no remove API).
    #[test]
    fn destroyed_cover_is_append_only() {
        let mut grid = OccupancyGrid::new();
        let a = key(1, 1, 0);
        let b = key(2, 2, 0);

        assert!(
            grid.destroyed_cover().is_empty(),
            "a fresh grid has no destroyed cover",
        );

        grid.mark_cover_destroyed(a);
        grid.mark_cover_destroyed(b);
        // Re-marking is a harmless no-op (set semantics) — still two cells.
        grid.mark_cover_destroyed(a);

        assert_eq!(grid.destroyed_cover().len(), 2, "two distinct cells marked");
        assert!(grid.destroyed_cover().contains(&a));
        assert!(grid.destroyed_cover().contains(&b));

        // A fresh build does NOT carry the set forward (occupancy is rebuilt fresh;
        // carrying destroyed cover across a rebuild is the caller's append — E1.7).
        let rebuilt = OccupancyGrid::build_from_occupancy_input(&OccupancyInput::new());
        assert!(
            rebuilt.destroyed_cover().is_empty(),
            "a rebuild starts with an empty destroyed-cover set",
        );
    }

    /// Out-of-range coordinates are handled gracefully — no panic, and they read as
    /// Open / empty / not-blocked. Probes negative and past-extent coordinates on
    /// every axis.
    #[test]
    fn out_of_range_coords_are_graceful() {
        let mut grid = OccupancyGrid::new();

        let negative = key(-1, 5, 0);
        let past_x = key(extent_i32(GRID_WIDTH), 0, 0);
        let past_y = key(0, extent_i32(GRID_HEIGHT), 0);
        let past_level = key(0, 0, MAX_LEVELS);

        for oob in [negative, past_x, past_y, past_level] {
            assert_eq!(grid.terrain(&oob), TerrainKind::Open);
            assert_eq!(grid.occupant(&oob), None);
            assert!(
                !grid.is_blocked(&oob),
                "an out-of-range cell must not block"
            );
            assert!(grid.slot(&oob).is_none());
            // Setting on an out-of-range key is a graceful no-op (no panic).
            grid.set_terrain(oob, TerrainKind::Wall);
            grid.set_occupant(oob, None);
            assert_eq!(grid.terrain(&oob), TerrainKind::Open);
        }
    }

    /// The grid spans the full 60×60×8 extent — the structural constants. The corner
    /// `(59, 59, 7)` is in-range (last valid slot) and `(60, 60, 8)` is out — pinning
    /// the STRUCTURAL dimensions (system definition, not balance tuning).
    #[test]
    fn grid_spans_full_extent() {
        assert_eq!(GRID_WIDTH, 60);
        assert_eq!(GRID_HEIGHT, 60);
        assert_eq!(MAX_LEVELS, 8);

        let grid = OccupancyGrid::new();
        let last = key(
            extent_i32(GRID_WIDTH) - 1,
            extent_i32(GRID_HEIGHT) - 1,
            MAX_LEVELS - 1,
        );
        assert!(
            grid.slot(&last).is_some(),
            "(59,59,7) is the last valid slot"
        );

        let past = key(extent_i32(GRID_WIDTH), extent_i32(GRID_HEIGHT), MAX_LEVELS);
        assert!(grid.slot(&past).is_none(), "(60,60,8) is out of range");
    }

    /// A later occupant placement at the same `(cell, level)` overwrites an earlier
    /// one — the pour applies placements in order. (Two occupants on one cell is the
    /// situation author's concern, E1.8; the grid just stores the last write.)
    #[test]
    fn later_occupant_placement_wins() {
        let mut world = World::new();
        let first = world.spawn_empty().id();
        let second = world.spawn_empty().id();
        let at = key(3, 3, 0);

        let input = OccupancyInput {
            terrain:   Vec::new(),
            occupants: vec![
                OccupantPlacement::new(at, first),
                OccupantPlacement::new(at, second),
            ],
        };
        let grid = OccupancyGrid::build_from_occupancy_input(&input);
        assert_eq!(
            grid.occupant(&at),
            Some(second),
            "the later placement at the same cell wins",
        );
    }
}
