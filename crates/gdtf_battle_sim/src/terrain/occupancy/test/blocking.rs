use super::{
    super::{OccupancyGrid, OccupancyInput, TerrainKind, TerrainPlacement},
    support::*,
};

#[test]
fn destroyed_cover_is_excluded_from_blocking() {
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

    grid.mark_cover_destroyed(smashed);
    assert!(
        !*grid.is_blocked(&smashed),
        "a destroyed cover cell must NOT block (C6)",
    );
    assert!(
        *grid.is_blocked(&intact),
        "an unrelated standing cover cell must still block",
    );
    assert!(*grid.is_cover_destroyed(&smashed));
    assert!(!*grid.is_cover_destroyed(&intact));
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

#[test]
fn destroyed_cover_is_append_only() {
    let mut grid = OccupancyGrid::new();
    let a = key(1, 1, 0);
    let b = key(2, 2, 0);

    assert!(
        grid.destroyed_cover().is_empty(),
        "a fresh grid has no destroyed cover",
    );

    grid.mark_cover_destroyed(a);
    grid.mark_cover_destroyed(b);
    grid.mark_cover_destroyed(a);

    assert_eq!(grid.destroyed_cover().len(), 2, "two distinct cells marked");
    assert!(grid.destroyed_cover().contains(&a));
    assert!(grid.destroyed_cover().contains(&b));

    let rebuilt =
        OccupancyGrid::build_from_occupancy_input(&OccupancyInput::new(), &no_stair_cells());
    assert!(
        rebuilt.destroyed_cover().is_empty(),
        "a rebuild starts with an empty destroyed-cover set",
    );
}
