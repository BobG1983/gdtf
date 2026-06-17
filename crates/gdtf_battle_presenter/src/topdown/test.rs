//! Unit tests for the top-down renderer px/coordinate bridge.

use gdtf_battle_sim::{Cell, Level};

use super::bridge::{CELL_PX, cell_to_world, z_for};

/// AC2 — `CELL_PX` is exactly 16.0, the single source of truth for cell size.
#[test]
fn cell_px_is_sixteen() {
    assert_eq!(
        CELL_PX.to_bits(),
        16.0_f32.to_bits(),
        "CELL_PX must be exactly 16.0 (one 16px source tile per cell)",
    );
}

/// AC3 — `cell_to_world` puts row 0 at the top (Bevy +Y up) and grows `x` with
/// `cell.x` by exactly `CELL_PX`; `z` is the per-level draw-z.
#[test]
fn cell_to_world_projects_row_zero_to_the_top() {
    let l0 = Level::new(0);

    // Origin maps to x == 0.
    assert_eq!(
        cell_to_world(Cell::new(0, 0), l0).x.to_bits(),
        0.0_f32.to_bits(),
        "cell (0,0) must project to world x == 0",
    );

    // Greater cell.y goes DOWN the grid => smaller (more negative) world y,
    // so row 0 sits ABOVE row 1.
    assert!(
        cell_to_world(Cell::new(0, 0), l0).y >= cell_to_world(Cell::new(0, 1), l0).y,
        "row 0 must be at least as near the top (greater y) as row 1",
    );

    // x grows by exactly CELL_PX per column; y is -CELL_PX per row.
    let p = cell_to_world(Cell::new(1, 2), l0);
    assert_eq!(
        p.x.to_bits(),
        (1.0 * CELL_PX).to_bits(),
        "cell.x == 1 must project to x == 1 * CELL_PX",
    );
    assert_eq!(
        p.y.to_bits(),
        (-2.0 * CELL_PX).to_bits(),
        "cell.y == 2 must project to y == -2 * CELL_PX",
    );

    // z is the per-level draw-z for L0.
    assert_eq!(p.z.to_bits(), z_for(l0).to_bits(), "z must equal z_for(L0)");
}

/// `z_for` is monotonic in the storey index so higher storeys draw in front.
#[test]
fn z_for_is_monotonic_in_storey() {
    assert!(
        z_for(Level::new(1)) > z_for(Level::new(0)),
        "a higher storey must draw at a greater z",
    );
}
