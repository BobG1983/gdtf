//! The editor's **in-memory paintable map model** — the authoritative store of painted
//! cells the click-to-paint flow writes (GTW-426 C2/C3).
//!
//! GTW-423 drew the canvas as a `width × height` grid of cells PRE-FILLED with the theme's
//! default-floor sprite; that fill is purely a default. GTW-426 lets the author PAINT a cell
//! with the active palette tile. This module owns the persistent record of those paints —
//! a sparse [`HashMap`] keyed by the sim's [`Cell`] ground-plane coordinate, valued by the
//! painted [`TileKey`]. An unpainted cell holds NO entry and renders the theme default-floor;
//! a painted cell holds its [`TileKey`] and renders that tile (C2).
//!
//! ## Why a sparse map keyed by [`Cell`]
//!
//! The model is sparse (only painted cells are stored) so the common case — a freshly-extented
//! grid the author has not yet touched — costs nothing, and it scales to the full `60 × 60`
//! drawable area without a dense per-cell allocation. Keying by the sim's own [`Cell`] newtype
//! (no-bare-types: the cell key is a domain coordinate, not a bare `IVec2` / index) makes the
//! model speak the SAME coordinate vocabulary the procgen assembly and the save/emit paths
//! (GTW-429 save gang / GTW-431 emit to sim / GTW-432 save `.prefab.ron`) read — it is the
//! reusable FOUNDATION those later tickets layer on, not a throwaway.
//!
//! ## State-scoped
//!
//! Inserted `OnEnter(Editing)` and removed `OnExit(Editing)` (bevy-traps #1, the
//! [`MapEditorSession`](crate::session::MapEditorSession) precedent), so every reader guards
//! with `Option<Res<…>>` / `run_if(resource_exists::<…>)`.
//!
//! ## Clamp (C3)
//!
//! A paint is only recorded for a cell INSIDE the current drawable extent — [`EditorMap::paint`]
//! takes the active [`GridSize`] and silently ignores an out-of-bounds cell, so a click off the
//! grid never writes the model. The canvas only spawns cells inside the extent, so in normal
//! play every paintable cell is in-bounds; the clamp is the model-level backstop that keeps the
//! stored set honest (the save/emit paths can trust every key is in the drawable area).

use bevy::{platform::collections::HashMap, prelude::*};
use gdtf_battle_sim::{
    Cell,
    level::{GridSize, TileKey},
};

/// The editor's **paintable map model** — the persistent record of which cells the author has
/// painted, keyed by the sim's [`Cell`] ground-plane coordinate (GTW-426 C2/C3).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1). Sparse: only PAINTED cells hold an entry (an unpainted cell renders the
/// theme default-floor). The authoritative editor paintable map — the FOUNDATION GTW-429 /
/// GTW-431 / GTW-432 read to save the gang / emit the assembled level / write the `.prefab.ron`.
///
/// The inner map is PRIVATE (no-bare-types rule 5): paints flow through [`EditorMap::paint`]
/// (which clamps to the drawable extent — C3) and are read through [`EditorMap::tile_at`] /
/// [`EditorMap::painted`], so the in-bounds invariant lives in one place and can never be
/// sidestepped by a direct map write.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct EditorMap {
    /// The painted cells: each entry maps a drawable-area [`Cell`] to the [`TileKey`] the
    /// author painted there. Cells with no entry render the theme default-floor.
    painted: HashMap<Cell, TileKey>,
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

    /// Paint `cell` with `tile`, IF the cell is inside the current drawable extent (C2 + C3).
    ///
    /// Records the paint in the sparse model so it PERSISTS (survives a later read and the
    /// save/emit paths — C2). Clamped to the drawable area (C3): a cell whose x/y falls outside
    /// `0..width` / `0..height` is silently ignored, so a click off the grid never writes the
    /// model. Returns `true` iff the paint was recorded (the cell was in-bounds) — the caller
    /// uses this to decide whether to redraw the cell sprite.
    pub fn paint(&mut self, cell: Cell, tile: TileKey, size: GridSize) -> bool {
        if !cell_in_bounds(cell, size) {
            return false;
        }
        self.painted.insert(cell, tile);
        true
    }

    /// The [`TileKey`] painted at `cell`, or [`None`] if the cell is unpainted (it renders the
    /// theme default-floor). The read the canvas redraw + the save/emit paths use.
    #[must_use]
    pub fn tile_at(&self, cell: Cell) -> Option<&TileKey> {
        self.painted.get(&cell)
    }

    /// Every painted `(cell, tile)` in the model — the enumeration the save/emit paths (GTW-429
    /// / GTW-431 / GTW-432) read to serialise the authored map. Iteration order is unspecified
    /// (a [`HashMap`]); the model is a SET of painted cells, not an ordered list.
    pub fn painted(&self) -> impl Iterator<Item = (&Cell, &TileKey)> {
        self.painted.iter()
    }

    /// How many cells the author has painted — `0` for a fresh (untouched) map. The count the
    /// paint/clamp test asserts.
    #[must_use]
    pub fn painted_count(&self) -> usize {
        self.painted.len()
    }
}

/// Whether `cell` falls inside the drawable extent `0..width` × `0..height` (C3).
///
/// The drawable area is the ground plane the canvas draws — cells `(0, 0)` through
/// `(width - 1, height - 1)`. A negative or over-extent coordinate is out of bounds (a click off
/// the grid). The 2D x/y plane only — z (levels) is out of the canvas's scope (GTW-423).
fn cell_in_bounds(cell: Cell, size: GridSize) -> bool {
    let width = i32::from(*size.width());
    let height = i32::from(*size.height());
    cell.x >= 0 && cell.x < width && cell.y >= 0 && cell.y < height
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::{
        Cell,
        level::{GridHeight, GridLevels, GridSize, GridWidth, TileKey},
    };

    use super::EditorMap;

    /// A small `4 × 4 × 1` drawable extent for the clamp tests, or a `1 × 1 × 1` fallback (the
    /// constructor is fallible; the fallback keeps the test panic-free per the workspace lints).
    fn small_size() -> GridSize {
        GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(1))
            .unwrap_or_else(|_| GridSize::default())
    }

    /// A throwaway tile key for the model writes (the model is value-agnostic about the key).
    fn key(id: &str) -> TileKey {
        TileKey::new(id.to_owned())
    }

    /// An in-bounds paint is recorded and read back by its [`Cell`]; a fresh model is empty.
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

        let recorded = map.paint(Cell::new(1, 2), key("brick"), small_size());
        assert!(recorded, "an in-bounds paint must be recorded (C2)");
        assert_eq!(map.painted_count(), 1, "one paint -> one entry");
        assert_eq!(
            map.tile_at(Cell::new(1, 2)),
            Some(&key("brick")),
            "the painted cell reads back its tile keyed by its Cell (C2)",
        );
    }

    /// Painting the same cell twice OVERWRITES (one entry, the latest tile) — the model is a map,
    /// not an append log.
    #[test]
    fn repaint_overwrites_the_same_cell() {
        let mut map = EditorMap::new();
        let cell = Cell::new(0, 0);
        assert!(map.paint(cell, key("a"), small_size()));
        assert!(map.paint(cell, key("b"), small_size()));
        assert_eq!(
            map.painted_count(),
            1,
            "repainting a cell must not add an entry"
        );
        assert_eq!(
            map.tile_at(cell),
            Some(&key("b")),
            "repainting must overwrite with the latest tile",
        );
    }

    /// Out-of-bounds cells (negative or past the extent on either axis) are rejected by the clamp
    /// — the model only ever holds in-bounds cells (C3).
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
                !map.paint(out, key("x"), size),
                "out-of-bounds cell {out:?} must be rejected (C3)",
            );
        }
        assert_eq!(
            map.painted_count(),
            0,
            "no out-of-bounds cell may enter the model (C3)"
        );
    }
}
