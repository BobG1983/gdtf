use gdtf_battle_sim::prelude::{Cell, CellLevel, Level};

pub(crate) fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(5, 6), Level::new(0))
}
