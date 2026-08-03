//! Compute the grid cell one step along facing.

use crate::{
    ganger::{Facing, Position},
    metric::{Cell, Level, SimPos, cell_center, pos_to_cell},
};

/// Cell (and same level) one step in the facing direction from `position`.
#[must_use]
pub fn faced_cell(position: &Position, facing: &Facing) -> (Cell, Level) {
    let (cell, level) = position.split();

    let stepped = *cell_center(cell, level) + *facing.forward_step();
    let (faced, _faced_level) = pos_to_cell(SimPos::new(stepped.x, stepped.y, stepped.z));

    (faced, level)
}
