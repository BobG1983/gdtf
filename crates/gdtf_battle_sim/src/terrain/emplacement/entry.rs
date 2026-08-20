//! The cells an emplacement's rotated entry sides sit on.

use bevy::prelude::Deref;

use super::state::{EmplacementEntrySides, EmplacementFacing};
use crate::{
    metric::{Cell, CellLevel, Level},
    terrain::{def::rotated_entry_sides, facing::TerrainFacing},
};

/// The cells an emplacement is entered from and left by, once its placed facing is applied.
#[derive(Deref, Debug, Clone, Default, PartialEq, Eq)]
pub struct EntryCells(Vec<CellLevel>);

impl EntryCells {
    /// Wrap the rotated entry cells.
    #[must_use]
    pub const fn new(cells: Vec<CellLevel>) -> Self {
        Self(cells)
    }
}

/// The cells this seat's rotated entry sides sit on. An emplacement naming no side has none.
///
/// An absent facing reads as the default one, which is how an unrotated seat is spelled.
#[must_use]
pub fn emplacement_entry_cells(
    seat: CellLevel,
    sides: Option<&EmplacementEntrySides>,
    facing: Option<&EmplacementFacing>,
) -> EntryCells {
    let Some(sides) = sides else {
        return EntryCells::new(Vec::new());
    };
    let turned = facing.map_or_else(TerrainFacing::default, |placed| **placed);
    let (cell, level) = seat.split();
    EntryCells::new(
        rotated_entry_sides(sides, turned)
            .into_iter()
            .map(|side| stepped(cell, level, side.cell_step()))
            .collect(),
    )
}

fn stepped(seat: Cell, level: Level, step: Cell) -> CellLevel {
    CellLevel::new(Cell::new(seat.x + step.x, seat.y + step.y), level)
}
