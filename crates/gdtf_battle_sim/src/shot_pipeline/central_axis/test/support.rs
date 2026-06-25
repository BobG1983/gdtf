//! Shared geometry-test helpers for the §1 central-axis tests.

use crate::{
    ganger::Position,
    metric::{Cell, CellLevel, Level},
};

/// A loose f32 tolerance for the geometry checks (`1/√2`, trig, normalisation
/// are not exactly representable).
pub(super) const TOL: f32 = 1.0e-5;

/// Build a `Position` at `(x, y)` on storey `level`.
pub(super) fn position(x: i32, y: i32, level: u8) -> Position {
    Position::new(CellLevel::new(Cell::new(x, y), Level::new(level)))
}
