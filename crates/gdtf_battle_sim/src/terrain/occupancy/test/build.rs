use bevy::ecs::world::World;

use super::{
    super::{
        GRID_HEIGHT, GRID_WIDTH, OccupancyGrid, OccupancyInput, OccupantPlacement, TerrainKind,
        TerrainPlacement,
    },
    support::*,
};
use crate::{cover::HeightBand, metric::MAX_LEVELS};

#[test]
fn build_from_input_populates_terrain_and_occupant_cell_by_cell() {
    let mut world = World::new();
    let alice = world.spawn_empty().id();
    let bob = world.spawn_empty().id();

    let wall_at = key(1, 2, 0);
    let cover_at = key(3, 4, 1);
    let alice_at = key(5, 6, 0);
    let bob_at = key(7, 8, 2);
    let empty_at = key(10, 10, 0);

    let input = OccupancyInput {
        terrain:   vec![
            TerrainPlacement::new(wall_at, TerrainKind::Wall),
            TerrainPlacement::new(cover_at, TerrainKind::Cover),
        ],
        occupants: vec![
            OccupantPlacement::new(alice_at, alice, HeightBand::High),
            OccupantPlacement::new(bob_at, bob, HeightBand::Low),
        ],
    };

    let grid = OccupancyGrid::build_from_occupancy_input(&input, &no_stair_cells());

    assert_eq!(
        grid.terrain(&wall_at),
        TerrainKind::Wall,
        "the wall slot must carry TerrainKind::Wall",
    );
    assert_eq!(
        grid.terrain(&cover_at),
        TerrainKind::Cover,
        "the cover slot must carry TerrainKind::Cover",
    );
    assert_eq!(
        grid.occupant(&alice_at),
        Some(alice),
        "alice's slot must hold alice's Entity handle",
    );
    assert_eq!(
        grid.occupant(&bob_at),
        Some(bob),
        "bob's slot must hold bob's Entity handle",
    );
    assert_eq!(
        grid.occupant_band(&alice_at),
        Some(HeightBand::High),
        "alice's slot must carry her placement band (poured with the occupant)",
    );
    assert_eq!(
        grid.occupant_band(&bob_at),
        Some(HeightBand::Low),
        "bob's slot must carry his placement band (poured with the occupant)",
    );
    assert_eq!(
        grid.terrain(&empty_at),
        TerrainKind::Open,
        "an untouched slot must default to Open",
    );
    assert_eq!(
        grid.occupant(&empty_at),
        None,
        "an untouched slot must have no occupant",
    );
    assert_eq!(
        grid.occupant_band(&empty_at),
        None,
        "an untouched slot must have no published band",
    );
    assert_eq!(grid.occupant(&wall_at), None);
    assert_eq!(grid.terrain(&alice_at), TerrainKind::Open);
}

#[test]
fn out_of_range_coords_are_graceful() {
    let mut grid = OccupancyGrid::new();

    let negative = key(-1, 5, 0);
    let past_x = key(extent_i32(GRID_WIDTH), 0, 0);
    let past_y = key(0, extent_i32(GRID_HEIGHT), 0);
    let past_level = key(0, 0, MAX_LEVELS);

    for oob in [negative, past_x, past_y, past_level] {
        assert_eq!(grid.terrain(&oob), TerrainKind::Open);
        assert_eq!(grid.occupant(&oob), None);
        assert!(
            !*grid.is_blocked(&oob),
            "an out-of-range cell must not block"
        );
        assert!(grid.slot(&oob).is_none());
        grid.set_terrain(oob, TerrainKind::Wall);
        grid.set_occupant(oob, None);
        assert_eq!(grid.terrain(&oob), TerrainKind::Open);
    }
}

#[test]
fn grid_spans_full_extent() {
    assert_eq!(GRID_WIDTH, 60);
    assert_eq!(GRID_HEIGHT, 60);
    assert_eq!(MAX_LEVELS, 8);

    let grid = OccupancyGrid::new();
    let last = key(
        extent_i32(GRID_WIDTH) - 1,
        extent_i32(GRID_HEIGHT) - 1,
        MAX_LEVELS - 1,
    );
    assert!(
        grid.slot(&last).is_some(),
        "(59,59,7) is the last valid slot"
    );

    let past = key(extent_i32(GRID_WIDTH), extent_i32(GRID_HEIGHT), MAX_LEVELS);
    assert!(grid.slot(&past).is_none(), "(60,60,8) is out of range");
}

#[test]
fn later_occupant_placement_wins() {
    let mut world = World::new();
    let first = world.spawn_empty().id();
    let second = world.spawn_empty().id();
    let at = key(3, 3, 0);

    let input = OccupancyInput {
        terrain:   Vec::new(),
        occupants: vec![
            OccupantPlacement::new(at, first, HeightBand::Low),
            OccupantPlacement::new(at, second, HeightBand::High),
        ],
    };
    let grid = OccupancyGrid::build_from_occupancy_input(&input, &no_stair_cells());
    assert_eq!(
        grid.occupant(&at),
        Some(second),
        "the later placement at the same cell wins",
    );
    assert_eq!(
        grid.occupant_band(&at),
        Some(HeightBand::High),
        "the winning placement's band wins too (occupant + band poured together)",
    );
}
