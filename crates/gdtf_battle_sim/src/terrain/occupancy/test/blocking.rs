use super::{
    super::{OccupancyGrid, OccupancyInput, TerrainKind, TerrainPlacement},
    support::*,
};

#[test]
fn a_cleared_cover_cell_is_excluded_from_blocking() {
    let input = OccupancyInput {
        terrain:   vec![
            TerrainPlacement::new(key(2, 2, 0), TerrainKind::Cover),
            TerrainPlacement::new(key(9, 9, 0), TerrainKind::Cover),
        ],
        occupants: Vec::new(),
    };
    let mut grid = OccupancyGrid::build_from_occupancy_input(&input, &no_stair_cells());

    let smashed = key(2, 2, 0);
    let intact = key(9, 9, 0);

    assert!(*grid.is_blocked(&smashed), "standing cover must block");
    assert!(*grid.is_blocked(&intact), "standing cover must block");

    grid.set_terrain(smashed, TerrainKind::Open);
    assert!(
        !*grid.is_blocked(&smashed),
        "a cell cleared to Open must NOT block, because blocking reads the kind standing in it",
    );
    assert!(
        *grid.is_blocked(&intact),
        "an unrelated standing cover cell must still block",
    );
}

#[test]
fn wall_blocks_open_does_not() {
    let input = OccupancyInput {
        terrain:   vec![TerrainPlacement::new(key(4, 4, 0), TerrainKind::Wall)],
        occupants: Vec::new(),
    };
    let grid = OccupancyGrid::build_from_occupancy_input(&input, &no_stair_cells());

    assert!(*grid.is_blocked(&key(4, 4, 0)), "a wall must block");
    assert!(
        !*grid.is_blocked(&key(0, 0, 0)),
        "an Open cell must not block",
    );
    assert!(*TerrainKind::Wall.blocks());
    assert!(*TerrainKind::Cover.blocks());
    assert!(!*TerrainKind::Open.blocks());
}
