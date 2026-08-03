use bevy::prelude::Deref;

use crate::{
    cover::CoverLedger,
    ganger::Direction,
    metric::{Cell, CellDistance, CellLevel},
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct EndsBehindCover(bool);

impl EndsBehindCover {
        const fn new(behind_cover: bool) -> Self {
        Self(behind_cover)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SuppressedMoveLegal(bool);

impl SuppressedMoveLegal {
        pub(super) const fn new(legal: bool) -> Self {
        Self(legal)
    }
}

fn chebyshev_xy(a: &CellLevel, b: &CellLevel) -> CellDistance {
    let dx = (a.x - b.x).unsigned_abs();
    let dy = (a.y - b.y).unsigned_abs();
    CellDistance::new(dx.max(dy))
}

fn ends_behind_cover(
    dest: &CellLevel,
    suppressor: &CellLevel,
    cover: &CoverLedger,
) -> EndsBehindCover {
    let dest_cell = dest.cell();
    let suppressor_cell = suppressor.cell();
    let Some(dir) = Direction::from_cells(dest_cell, suppressor_cell) else {
        return EndsBehindCover::new(false);
    };
    let step = dir.cell_step();
    let toward = Cell::new(dest_cell.x + step.x, dest_cell.y + step.y);
    EndsBehindCover::new(cover.peek(&CellLevel::new(toward, dest.level())).is_some())
}

pub(super) fn suppressed_move_legal(
    start: &CellLevel,
    dest: &CellLevel,
    suppressor: &CellLevel,
    cover: &CoverLedger,
) -> SuppressedMoveLegal {
    let farther = chebyshev_xy(dest, suppressor) > chebyshev_xy(start, suppressor);
    SuppressedMoveLegal::new(farther && *ends_behind_cover(dest, suppressor, cover))
}
