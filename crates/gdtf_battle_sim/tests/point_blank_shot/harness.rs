//! The one point-blank cell helper shared by BOTH halves (the pure march sweep
//! and the full-app volley) — deliberately tiny.

use gdtf_battle_sim::prelude::{Cell, CellLevel, Level};

/// The shooter's cell (interior of the grid so every facing has an adjacent cell).
pub(crate) fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(5, 6), Level::new(0))
}
