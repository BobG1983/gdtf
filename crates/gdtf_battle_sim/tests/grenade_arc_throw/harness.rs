use gdtf_battle_sim::metric::{Cell, CellLevel, Level};

pub(crate) const PLAYER: u8 = 0;

pub(crate) const TEST_VIEW_RANGE: u16 = 30;

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

pub(crate) fn at_level(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}
