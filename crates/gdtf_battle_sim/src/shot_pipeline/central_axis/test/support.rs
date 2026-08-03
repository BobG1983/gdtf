use crate::{
    ganger::Position,
    metric::{Cell, CellLevel, Level},
};

pub(super) const TOL: f32 = 1.0e-5;

pub(super) fn position(x: i32, y: i32, level: u8) -> Position {
    Position::new(CellLevel::new(Cell::new(x, y), Level::new(level)))
}
