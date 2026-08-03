use gdtf_battle_sim::prelude::{Cell, Level};

use super::super::projection::{
    CELL_PX, GANGER_Z_BIAS, Layer, cell_to_world, cell_to_world_layered, z_for,
};

#[test]
fn cell_px_is_sixteen() {
    assert_eq!(
        CELL_PX.to_bits(),
        16.0_f32.to_bits(),
        "CELL_PX must be exactly 16.0 (one 16px source tile per cell)",
    );
}

#[test]
fn cell_to_world_projects_row_zero_to_the_top() {
    let l0 = Level::new(0);

    assert_eq!(
        cell_to_world(Cell::new(0, 0), l0).x.to_bits(),
        0.0_f32.to_bits(),
        "cell (0,0) must project to world x == 0",
    );

    assert!(
        cell_to_world(Cell::new(0, 0), l0).y >= cell_to_world(Cell::new(0, 1), l0).y,
        "row 0 must be at least as near the top (greater y) as row 1",
    );

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

    assert_eq!(p.z.to_bits(), z_for(l0).to_bits(), "z must equal z_for(L0)");
}

#[test]
fn z_for_is_monotonic_in_storey() {
    assert!(
        z_for(Level::new(1)) > z_for(Level::new(0)),
        "a higher storey must draw at a greater z",
    );
}

#[test]
fn actor_layer_draws_above_terrain_within_its_storey() {
    let l0 = Level::new(0);
    let cell = Cell::new(3, 4);

    let terrain = cell_to_world_layered(cell, l0, Layer::Terrain);
    let actor = cell_to_world_layered(cell, l0, Layer::Actor);
    let highlight = cell_to_world_layered(cell, l0, Layer::Highlight);

    assert_eq!(
        terrain,
        cell_to_world(cell, l0),
        "the Terrain layer must equal the bare cell_to_world projection (zero bias)",
    );

    assert_eq!(actor.x.to_bits(), terrain.x.to_bits(), "Actor x unchanged");
    assert_eq!(actor.y.to_bits(), terrain.y.to_bits(), "Actor y unchanged");

    assert!(
        terrain.z < actor.z && actor.z < highlight.z,
        "draw order must be terrain ({}) < actor ({}) < highlight ({})",
        terrain.z,
        actor.z,
        highlight.z,
    );

    assert_eq!(
        (actor.z - terrain.z).to_bits(),
        GANGER_Z_BIAS.to_bits(),
        "the Actor layer lifts by exactly GANGER_Z_BIAS",
    );

    assert!(
        actor.z < z_for(Level::new(1)),
        "an Actor at level 0 must draw strictly below level 1's band (z {} < {})",
        actor.z,
        z_for(Level::new(1)),
    );
}
