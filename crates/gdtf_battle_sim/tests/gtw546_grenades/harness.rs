//! The cell/level helpers + faction/view-range constants shared by BOTH the pure
//! arc-march half and the full-app throw half — deliberately tiny (the sole
//! cross-half items).

use gdtf_battle_sim::metric::{Cell, CellLevel, Level};

/// The single faction every fixture ganger belongs to. One faction (no opponents) means no
/// setup-time AI / reaction fire corrupts the baselines, and the blast striking teammates IS
/// the faction-blind friendly-fire property (the gtw541 harness ruling).
pub(crate) const PLAYER: u8 = 0;

/// A view range comfortably covering the whole scene.
pub(crate) const TEST_VIEW_RANGE: u16 = 30;

/// A ground-floor `(cell, level)` key.
pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// An `(x, y)` key on a given storey.
pub(crate) fn at_level(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}
