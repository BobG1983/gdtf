//! The faced-cell geometry implementation — [`faced_cell`]. See the module docs
//! (`super`) for the diagonal center-offset derivation.

use crate::{
    ganger::{Facing, Position},
    metric::{Cell, Level, SimPos, cell_center, pos_to_cell},
};

/// The ground cell a shooter **faces** — one step along its [`Facing`], on the
/// shooter's own storey.
///
/// Given the shooter's [`Position`] (a `(cell, level)` key) and its [`Facing`],
/// returns the faced ([`Cell`], [`Level`]): the cell one unit step along the facing,
/// with the shooter's [`Level`] preserved unchanged (the step is horizontal —
/// [`Direction::forward_step`](crate::ganger::Direction::forward_step) has `z = 0` —
/// so the faced cell is on the shooter's own storey). This is the cell whose cover
/// height the §1a brace gate reads (`docs/combat/resolution.md` line 26: "+30 when
/// the faced cell's cover height suits the stance").
///
/// The faced cell is derived through the [`crate::metric`] sim-unit path so the
/// diagonal case is correct: it steps from [`cell_center`] (the `+0.5` x/y centering)
/// by the **unit** forward step, then [`pos_to_cell`] floors. A diagonal's `0.707`
/// component therefore crosses the cell boundary (`0.5 + 0.707 = 1.207 → floor 1`)
/// and advances by one cell on each spanned axis — which a corner-based derivation
/// (`0.0 + 0.707 → floor 0`) would not. **Zero pixels** — sim-unit voxel coords only.
#[must_use]
pub fn faced_cell(position: &Position, facing: &Facing) -> (Cell, Level) {
    // (1) Split the shooter's (cell, level) key — the canonical CellLevel::split
    //     (GTW-565; Position derefs to CellLevel). The shooter's own storey is
    //     preserved unchanged in the return (AC3): the step is horizontal (z = 0),
    //     so the faced cell is on the same level.
    let (cell, level) = position.split();

    // (2)-(4) Step from the cell CENTER by the facing's UNIT forward step, then floor
    //         back to a cell. Stepping from the center (not the corner) is what makes a
    //         0.707 diagonal cross the boundary; SimPos derefs to Vec3 for the add.
    let stepped = *cell_center(cell, level) + *facing.forward_step();
    let (faced, _faced_level) = pos_to_cell(SimPos::new(stepped.x, stepped.y, stepped.z));

    // (5) The faced cell, on the shooter's OWN level (the forward step is horizontal).
    (faced, level)
}
