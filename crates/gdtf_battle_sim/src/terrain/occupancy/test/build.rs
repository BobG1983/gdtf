//! Grid build/pour from `OccupancyInput` + the extent/bounds structure.

use bevy::ecs::world::World;

use super::{
    super::{
        GRID_HEIGHT, GRID_WIDTH, OccupancyGrid, OccupancyInput, OccupantPlacement, TerrainKind,
        TerrainPlacement,
    },
    support::*,
};
use crate::{cover::HeightBand, metric::MAX_LEVELS};

/// C8(a) — `build_from_occupancy_input` from a HAND-BUILT input (entities
/// spawned in a real Bevy `World`, terrain placed) populates terrain + occupant
/// slots correctly, asserted CELL-BY-CELL.
///
/// Spawns two real entities, hand-builds an `OccupancyInput` with a wall, a
/// cover, and those two occupants at distinct `(cell, level)`s, builds the grid,
/// then walks every authored slot and asserts BOTH its terrain marker and its
/// occupant handle, plus that an untouched slot is `Open` / empty. The occupant
/// is the spawned `Entity` handle, never a numeric id (GTW-10 / GTW-12).
#[test]
fn build_from_input_populates_terrain_and_occupant_cell_by_cell() {
    // Real entities from a real World (the C8(a) requirement).
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

    // Terrain, cell by cell.
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
    // Occupants, cell by cell — the exact spawned Entity handles.
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
    // The silhouette band is poured TOGETHER with the occupant (GTW-304): a placed
    // occupant must carry its band so the march can strike it.
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
    // A slot the situation never touched is Open with no occupant.
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
    // A terrain slot carries no occupant and an occupant slot is Open terrain —
    // the two facts are independent per slot.
    assert_eq!(grid.occupant(&wall_at), None);
    assert_eq!(grid.terrain(&alice_at), TerrainKind::Open);
}

/// Out-of-range coordinates are handled gracefully — no panic, and they read as
/// Open / empty / not-blocked. Probes negative and past-extent coordinates on
/// every axis.
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
        // Setting on an out-of-range key is a graceful no-op (no panic).
        grid.set_terrain(oob, TerrainKind::Wall);
        grid.set_occupant(oob, None);
        assert_eq!(grid.terrain(&oob), TerrainKind::Open);
    }
}

/// The grid spans the full 60×60×8 extent — the structural constants. The corner
/// `(59, 59, 7)` is in-range (last valid slot) and `(60, 60, 8)` is out — pinning
/// the STRUCTURAL dimensions (system definition, not balance tuning).
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

/// A later occupant placement at the same `(cell, level)` overwrites an earlier
/// one — the pour applies placements in order. (Two occupants on one cell is the
/// situation author's concern, E1.8; the grid just stores the last write.)
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
