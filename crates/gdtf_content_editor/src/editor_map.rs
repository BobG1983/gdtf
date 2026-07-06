//! The editor's **in-memory paintable map model** — the authoritative store of painted
//! cells the click-to-paint flow writes (GTW-426 C2/C3), extended to be **level-aware** so the
//! vertical placement rules can reason about a cell's neighbour one storey up (GTW-430 C1),
//! and swept onto the UUID-keyed terrain model (GTW-495): a painted cell holds the
//! [`TerrainUuid`] of the terrain definition placed there.
//!
//! ## Why a sparse map keyed by [`CellLevel`]
//!
//! The model is sparse (only painted cells are stored) so the common case — a freshly-extented
//! grid the author has not yet touched — costs nothing, and it scales to the full `60 × 60 × 8`
//! drawable volume without a dense per-cell allocation. Keying by the sim's own [`CellLevel`]
//! newtype (no-bare-types: the cell key is a domain coordinate) makes the model speak the SAME
//! coordinate vocabulary the procgen assembly and the save/emit paths read, and valuing it by
//! [`TerrainUuid`] (the new model's stable terrain key) makes a painted cell carry the same
//! reference a [`PrefabSpec`](gdtf_battle_sim::level::PrefabSpec) placement does.
//!
//! ## Ground-plane (`L0`) conveniences
//!
//! The GTW-423 canvas draws only the x/y GROUND plane (no storey selector — z is out of the
//! canvas's scope), so it paints/reads at [`Level`]`(0)`. The [`EditorMap::paint`] /
//! [`EditorMap::tile_at`] / [`EditorMap::painted`] methods keep their `Cell` signatures and
//! operate on `L0`; the level-aware [`EditorMap::paint_at`] / [`EditorMap::tile_at_level`] /
//! [`EditorMap::clear`] methods are what the GTW-430 vertical placement rule uses to read/clear
//! the cell one storey up (C1).
//!
//! ## State-scoped
//!
//! Inserted `OnEnter(Editing)` and removed `OnExit(Editing)` (bevy-traps #1, the
//! [`MapEditorSession`](crate::session::MapEditorSession) precedent), so every reader guards
//! with `Option<Res<…>>` / `run_if(resource_exists::<…>)`.
//!
//! ## Clamp (C3)
//!
//! A paint is only recorded for a cell INSIDE the current drawable extent — [`EditorMap::paint_at`]
//! takes the active [`GridSize`] and silently ignores an out-of-bounds `(cell, level)`, so a click
//! off the grid (or a level past the extent) never writes the model.

use bevy::{platform::collections::HashMap, prelude::*};
use gdtf_battle_sim::{
    level::GridSize,
    metric::{CellLevel, Level},
    prelude::Cell,
    terrain::def::TerrainUuid,
};

/// The ground storey index (`L0`) — the storey the GTW-423 canvas draws and the
/// [`EditorMap`]'s `Cell`-keyed conveniences operate on.
///
/// A named constant (not a magic `0`) so the ground-plane assumption the canvas encodes is
/// readable at every call site; the canvas has no storey selector, so it always paints `L0`.
pub(crate) const GROUND_LEVEL: Level = Level::new(0);

/// The editor's **paintable map model** — the persistent record of which `(cell, storey)` the
/// author has painted, keyed by the sim's [`CellLevel`] coordinate, valued by the painted
/// [`TerrainUuid`] (GTW-426 C2/C3; level-aware since GTW-430; UUID-keyed since GTW-495).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1). Sparse: only PAINTED cells hold an entry (an unpainted cell renders the
/// theme default-floor). The authoritative editor paintable map — the FOUNDATION the save path
/// (GTW-432) reads to write the `.prefab.ron`.
///
/// The inner map is PRIVATE (no-bare-types rule 5): paints flow through [`EditorMap::paint_at`]
/// (which clamps to the drawable extent — C3) and are read through [`EditorMap::tile_at_level`] /
/// [`EditorMap::painted`], so the in-bounds invariant lives in one place. The `Cell`-signature
/// [`EditorMap::paint`] / [`EditorMap::tile_at`] are ground-plane (`L0`) conveniences for the
/// canvas.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct EditorMap {
    /// The painted cells: each entry maps a drawable-volume [`CellLevel`] to the [`TerrainUuid`]
    /// the author painted there. Cells with no entry render the theme default-floor.
    painted: HashMap<CellLevel, TerrainUuid>,
}

impl EditorMap {
    /// Build an empty map model (nothing painted yet) — the `OnEnter(Editing)` seed and the
    /// test ctor. Every cell starts unpainted (renders the theme default-floor).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            painted: HashMap::new(),
        }
    }

    /// Paint the GROUND cell `(cell, L0)` with `tile`, IF in bounds — the canvas's ground-plane
    /// convenience (the GTW-423 canvas has no storey selector, so it always paints `L0`).
    ///
    /// Delegates to [`EditorMap::paint_at`] at `GROUND_LEVEL`; returns `true` iff the paint was
    /// recorded (the cell was in-bounds).
    pub fn paint(&mut self, cell: Cell, tile: TerrainUuid, size: GridSize) -> bool {
        self.paint_at(CellLevel::new(cell, GROUND_LEVEL), tile, size)
    }

    /// Paint the `(cell, level)` slot with `tile`, IF the slot is inside the current drawable
    /// volume (C2 + C3) — the level-aware write the placement rule (GTW-430) uses.
    ///
    /// Records the paint in the sparse model so it PERSISTS (survives a later read and the
    /// save path — C2). Clamped to the drawable volume (C3): a slot whose x/y/level falls
    /// outside `0..width` / `0..height` / `0..levels` is silently ignored. Returns `true` iff the
    /// paint was recorded (the slot was in-bounds).
    pub fn paint_at(&mut self, slot: CellLevel, tile: TerrainUuid, size: GridSize) -> bool {
        if !slot_in_bounds(slot, size) {
            return false;
        }
        self.painted.insert(slot, tile);
        true
    }

    /// Remove any paint at the `(cell, level)` slot, returning the [`TerrainUuid`] that was there
    /// (if any) — the level-aware clear the GTW-430 ladder rule uses to AUTO-CLEAR a slab one
    /// storey up (C1). A no-op on an already-unpainted slot.
    pub fn clear(&mut self, slot: CellLevel) -> Option<TerrainUuid> {
        self.painted.remove(&slot)
    }

    /// The [`TerrainUuid`] painted at the GROUND cell `(cell, L0)`, or [`None`] if unpainted (it
    /// renders the theme default-floor) — the canvas redraw + save ground-plane read.
    ///
    /// Delegates to [`EditorMap::tile_at_level`] at `GROUND_LEVEL`.
    #[must_use]
    pub fn tile_at(&self, cell: Cell) -> Option<TerrainUuid> {
        self.tile_at_level(CellLevel::new(cell, GROUND_LEVEL))
    }

    /// The [`TerrainUuid`] painted at the `(cell, level)` slot, or [`None`] if the slot is
    /// unpainted — the level-aware read the GTW-430 placement rule uses to inspect the cell one
    /// storey up.
    #[must_use]
    pub fn tile_at_level(&self, slot: CellLevel) -> Option<TerrainUuid> {
        self.painted.get(&slot).copied()
    }

    /// Every painted `(slot, tile)` in the model — the enumeration the save path (GTW-432) reads
    /// to serialise the authored map. Iteration order is unspecified (a [`HashMap`]); the model
    /// is a SET of painted slots, not an ordered list.
    pub fn painted(&self) -> impl Iterator<Item = (&CellLevel, &TerrainUuid)> {
        self.painted.iter()
    }

    /// How many slots the author has painted — `0` for a fresh (untouched) map. The count the
    /// paint/clamp tests assert.
    #[must_use]
    pub fn painted_count(&self) -> usize {
        self.painted.len()
    }
}

/// Whether `slot` falls inside the drawable volume `0..width` × `0..height` × `0..levels` (C3).
fn slot_in_bounds(slot: CellLevel, size: GridSize) -> bool {
    let width = i32::from(*size.width());
    let height = i32::from(*size.height());
    let levels = i32::from(*size.levels());
    slot.x >= 0
        && slot.x < width
        && slot.y >= 0
        && slot.y < height
        && slot.z >= 0
        && slot.z < levels
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::{
        level::{GridHeight, GridLevels, GridSize, GridWidth},
        metric::{CellLevel, Level},
        prelude::Cell,
        terrain::def::TerrainUuid,
    };

    use super::EditorMap;

    /// A small `4 × 4 × 2` drawable volume for the clamp tests, or a `1 × 1 × 1` fallback (the
    /// constructor is fallible; the fallback keeps the test panic-free per the workspace lints).
    fn small_size() -> GridSize {
        GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(2))
            .unwrap_or_else(|_| GridSize::default())
    }

    /// A throwaway terrain key for the model writes (the model is value-agnostic about the key).
    const fn key(n: u128) -> TerrainUuid {
        TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
    }

    /// An in-bounds GROUND paint is recorded and read back by its [`Cell`]; a fresh model is empty.
    #[test]
    fn in_bounds_paint_is_recorded_and_read_back() {
        let mut map = EditorMap::new();
        assert_eq!(
            map.painted_count(),
            0,
            "a fresh model holds nothing painted"
        );
        assert!(
            map.tile_at(Cell::new(1, 1)).is_none(),
            "unpainted cell reads None"
        );

        let recorded = map.paint(Cell::new(1, 2), key(0xAA), small_size());
        assert!(recorded, "an in-bounds paint must be recorded (C2)");
        assert_eq!(map.painted_count(), 1, "one paint -> one entry");
        assert_eq!(
            map.tile_at(Cell::new(1, 2)),
            Some(key(0xAA)),
            "the painted cell reads back its tile keyed by its Cell (C2)",
        );
    }

    /// Painting the same cell twice OVERWRITES (one entry, the latest tile) — the model is a map,
    /// not an append log.
    #[test]
    fn repaint_overwrites_the_same_cell() {
        let mut map = EditorMap::new();
        let cell = Cell::new(0, 0);
        assert!(map.paint(cell, key(0x1), small_size()));
        assert!(map.paint(cell, key(0x2), small_size()));
        assert_eq!(
            map.painted_count(),
            1,
            "repainting a cell must not add an entry"
        );
        assert_eq!(
            map.tile_at(cell),
            Some(key(0x2)),
            "repainting must overwrite with the latest tile",
        );
    }

    /// Out-of-bounds cells (negative or past the extent on either ground axis) are rejected by the
    /// clamp — the model only ever holds in-bounds cells (C3).
    #[test]
    fn out_of_bounds_paint_is_clamped_out() {
        let mut map = EditorMap::new();
        let size = small_size();
        let width = i32::from(*size.width());
        let height = i32::from(*size.height());
        for out in [
            Cell::new(-1, 0),
            Cell::new(0, -1),
            Cell::new(width, 0),
            Cell::new(0, height),
            Cell::new(width, height),
        ] {
            assert!(
                !map.paint(out, key(0x9), size),
                "out-of-bounds cell {out:?} must be rejected (C3)",
            );
        }
        assert_eq!(
            map.painted_count(),
            0,
            "no out-of-bounds cell may enter the model (C3)"
        );
    }

    /// A level-aware paint/read round-trips on its own storey, distinct from the ground plane;
    /// a level PAST the extent is clamped out (C3).
    #[test]
    fn level_aware_paint_is_storey_scoped_and_clamped() {
        let mut map = EditorMap::new();
        let size = small_size(); // 4 x 4 x 2 -> valid levels are 0 and 1.
        let ground = CellLevel::new(Cell::new(1, 1), Level::new(0));
        let above = CellLevel::new(Cell::new(1, 1), Level::new(1));

        assert!(map.paint_at(ground, key(0x10), size), "L0 paint records");
        assert!(map.paint_at(above, key(0x20), size), "L1 paint records");
        assert_eq!(
            map.painted_count(),
            2,
            "the same x/y on two storeys are two distinct slots",
        );
        assert_eq!(map.tile_at_level(ground), Some(key(0x10)));
        assert_eq!(map.tile_at_level(above), Some(key(0x20)));

        // A storey past the 2-level extent is out of bounds (C3).
        let over = CellLevel::new(Cell::new(1, 1), Level::new(2));
        assert!(
            !map.paint_at(over, key(0x30), size),
            "L2 is past the extent"
        );
        assert_eq!(
            map.painted_count(),
            2,
            "no over-extent storey enters the model"
        );

        // Clearing a slot removes exactly it.
        assert_eq!(
            map.clear(above),
            Some(key(0x20)),
            "clear returns the removed tile"
        );
        assert_eq!(map.painted_count(), 1, "clear removes one entry");
        assert!(
            map.tile_at_level(above).is_none(),
            "the cleared slot reads None"
        );
        assert!(
            map.clear(above).is_none(),
            "clearing an empty slot is a no-op"
        );
    }
}
